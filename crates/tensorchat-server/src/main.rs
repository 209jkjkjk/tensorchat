//! TensorChat server entry point.
//!
//! Startup only — the router and every module it wires together live in the
//! library half of this crate, so they can be exercised by integration tests
//! without a live process.

use std::sync::Arc;
use std::time::Duration;

use tensorchat_server::cli::{self, Command};
use tensorchat_server::{AppState, Config, Shared, build_router};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env().map_err(|e| format!("configuration error: {e}"))?;

    // Dispatch before starting a runtime. The operator commands are synchronous
    // database work with no server behind them, and spinning up tokio to print
    // an invite link would be pure ceremony.
    let command = cli::parse(std::env::args().skip(1)).map_err(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    })?;

    if !matches!(command, Command::Serve) {
        let store = cli::open_store(&cfg.db_path)?;
        match cli::run(&store, &cfg, command) {
            Ok(message) => {
                println!("{}", message.trim_end());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }

    serve(cfg)
}

#[tokio::main]
async fn serve(cfg: Config) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tensorchat_server=info,tower_http=warn".into()),
        )
        .compact()
        .init();

    // Create the directories we own before opening anything inside them, so a
    // fresh checkout runs with no setup step.
    if let Some(parent) = cfg.db_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&cfg.blob_dir)?;

    let store = tensorchat_store::Store::open(&cfg.db_path)?;
    tracing::info!(db = %cfg.db_path.display(), "database ready");

    // A workspace nobody can sign in to fails silently otherwise: the server
    // comes up, serves a login page, and refuses every credential because there
    // are none. Say so, and name the command that fixes it.
    cli::warn_if_unreachable(&store, &cfg);

    // Web Push needs a stable VAPID keypair, minted into the database on first
    // run. A failure here disables push rather than stopping the server: chat
    // works without notifications, and refusing to boot over them would be a
    // poor trade.
    let vapid = if cfg.push_contact.is_empty() {
        tracing::info!("web push disabled (TC_PUSH_CONTACT is empty)");
        None
    } else {
        match tensorchat_server::push::Vapid::load(&store, &cfg.push_contact) {
            Ok(v) => {
                tracing::info!("web push enabled");
                Some(v)
            }
            Err(e) => {
                tracing::warn!(error = %e, "web push disabled");
                None
            }
        }
    };

    let st: Shared = Arc::new(AppState::new(cfg.clone(), store).with_push(vapid));
    spawn_maintenance(st.clone());

    let app = build_router(st.clone());
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, "tensorchat listening");

    axum::serve(
        listener,
        // `ConnectInfo` gives the pre-auth rate limiter a client address.
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("shutting down");
    Ok(())
}

/// How long a spent invite lingers before housekeeping drops it.
///
/// Not zero, because an administrator asking "did that link ever get used?" a
/// week later deserves an answer. Expiry is enforced at redemption regardless,
/// so this only governs when the row stops taking up space.
const INVITE_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// Periodic housekeeping: expire sessions and invites, apply retention,
/// checkpoint the WAL, and refresh planner statistics.
fn spawn_maintenance(st: Shared) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(3600));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Run immediately: an operator restarting to apply a shorter period
        // should not have to wait an hour before it takes effect.
        run_maintenance(&st).await;
        tick.tick().await;
        loop {
            tick.tick().await;
            run_maintenance(&st).await;
        }
    });
}

async fn run_maintenance(st: &Shared) {
    let now = tensorchat_core::now_ms();
    let retention = st.cfg.retention_ms;
    let r = st
        .db(move |s| {
            let purged = s.purge_expired_sessions(now)?;
            let invites = s.purge_expired_invites(now.saturating_sub(INVITE_RETENTION_MS))?;
            let retention = match retention {
                Some(age) => Some(s.purge_retention(
                    tensorchat_core::Id::floor_for_ms(now.saturating_sub(age)),
                    now.saturating_sub(age),
                    now,
                )?),
                None => None,
            };
            s.maintenance()?;
            Ok((purged + invites, retention))
        })
        .await;
    let Ok((expired, retention)) = r else {
        tracing::warn!("maintenance failed");
        return;
    };
    if let Some(purge) = retention {
        for channel in purge.channels {
            st.hub
                .broadcast_frame(channel, &tensorchat_core::ServerFrame::ChanDel { channel });
            st.hub.unsubscribe_channel(channel);
        }
        for channel in purge.pruned_channels {
            if let Ok(channel) = st.db(move |s| s.channel(channel)).await {
                st.hub
                    .broadcast_frame(channel.id, &tensorchat_core::ServerFrame::Chan { channel });
            }
        }
    }
    remove_queued_blobs(st).await;
    if expired > 0 {
        tracing::info!(purged = expired, "maintenance: expired rows");
    }
}

async fn remove_queued_blobs(st: &Shared) {
    let Ok(paths) = st.db(|s| s.pending_blob_deletions(1000)).await else {
        return;
    };
    for rel in paths {
        // The upload endpoint creates decimal file names. Never turn a corrupt
        // database row into filesystem traversal during cleanup.
        if rel.is_empty() || rel.contains(['/', '\\', '.']) {
            continue;
        }
        let path = st.cfg.blob_dir.join(&rel);
        match tokio::fs::remove_file(path).await {
            Ok(()) => {
                let done = rel.clone();
                let _ = st.db(move |s| s.acknowledge_blob_deletion(&done)).await;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let done = rel.clone();
                let _ = st.db(move |s| s.acknowledge_blob_deletion(&done)).await;
            }
            Err(e) => tracing::warn!(path = %rel, error = %e, "blob deletion will retry"),
        }
    }
}

/// Resolve on SIGINT or SIGTERM so in-flight requests finish before exit.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            // Without SIGTERM we can still be stopped by Ctrl-C.
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
