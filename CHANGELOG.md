# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Keep the message viewport pinned to the true bottom while row heights are measured, including when opening a channel or receiving a new message.
- Re-measure message rows when an attached image finishes loading, so image decoding cannot leave the viewport short of the bottom.

- 无频道的工作区增加首次进入引导，提供加入频道和创建频道入口，并提示聊天记录保留策略。
- 注册时增加确认密码输入，避免两次输入不一致仍提交注册请求。
- 用户可以主动退出公开频道、私有频道和私聊；退出私聊后再次发起即可重新加入原会话。
- 移动端消息操作栏在点击消息外部或完成操作后自动收起。

- 移动端默认隐藏消息操作栏，点按消息时只显示当前消息的操作。
- 移动端增加频道栏的打开与收起按钮，并为成员列表增加关闭按钮。

- 注册页明确标注用户名不可使用中文，并将显示名称、浏览频道分别改为昵称、加入频道。
- 附件可单独作为一条消息发送，默认单文件上传上限从 25 MiB 提高至 100 MiB；可通过 `TC_MAX_UPLOAD` 调整。

- 自动清理默认保留七天：长期无消息的频道会连同记录和附件删除；仍活跃频道只清理过期消息，并保留一条会更新的清理提示。用 `TC_RETENTION=1m` 可测试，`TC_RETENTION=0` 可关闭。

- 私聊和群聊也可从界面归档；归档后从频道列表隐藏，消息记录仍按保留期规则处理。

- **中文界面与可配置站点名称。** 前端可见文案现使用中文；设置
  `TC_SITE_NAME` 可更改网页标题、登录页、侧栏和通知中显示的名称。

### Changed

- **Channel names accept Unicode.** Names may now contain spaces,
  Chinese and other non-Latin text, and emoji. Control characters and blank
  names remain invalid.

## [0.3.0] — 2026-08-05

### Added

- **Single sign-on through an OpenID Connect provider.** Set `TC_OIDC_ISSUER`
  and the three values beside it, and the login screen grows a "Sign in with …"
  button. Endpoints come from the issuer's discovery document, so any conforming
  provider works and nothing in the code names one. Leave it unset and the
  server behaves exactly as before.

  Accounts are created on first sign-in, taking their handle from the provider's
  `preferred_username` and numbering it if it is taken. Identities are keyed on
  the issuer and subject rather than on an email address, so there is no way to
  claim an existing account by controlling an address — and no way to attach a
  provider to an account that already has a password. Passwords keep working
  alongside; pair it with `TC_OPEN_REGISTRATION=false` for a provider-only
  workspace. Deactivating an account closes this route too.

## [0.2.0] — 2026-07-30

First release published to crates.io, as `tensorchat-core`, `tensorchat-store`
and `tensorchat-server`. The installed binary is named `tensorchat`.

### Changed

- **The crates were renamed.** `tc-core` and `tc-server` are names other people
  already hold on crates.io — `tc-server` since 2024, and `tc-core` by way of
  `tc_core`, since the registry treats hyphens and underscores as one name. The
  crates are now `tensorchat-core`, `tensorchat-store` and `tensorchat-server`,
  and the binary they build is `tensorchat` rather than `tc-server`. Anyone
  running the 0.1.0 binary or the Docker image should expect the command name
  and the `RUST_LOG` target (`tensorchat_server=info`) to have changed with it.

### Added

- **Operator console** — `tensorchat invite`, `promote` and `demote`, run against
  the database rather than the API. Closes the gap where a workspace started
  with `TC_OPEN_REGISTRATION=false` had nobody who could mint the first invite:
  the documented workaround was to start open, register, and restart closed,
  which left a window in which anyone who found the address became the
  administrator. `invite` mints a link through the existing invite machinery, so
  the first person picks their own handle and password and the ordinary
  "first human becomes administrator" rule does the rest. `promote` replaces the
  `sqlite3 "UPDATE users SET admin = 1"` the README used to recommend. The
  commands need no authentication because anyone able to run them can already
  read the database; they add no network surface and no new credential type.
  The server now also reports at startup when nobody can sign in, or when there
  is no active administrator, and names the command that fixes it.
- **Web Push and PWA** — mentions and direct messages now notify with no tab
  open. The push itself carries **no payload**: the server sends an empty
  VAPID-signed message and the service worker fetches the content from
  `/api/me/notifications` on the same origin, so message bodies never pass
  through a third-party push service. `POST /api/push/subscribe`,
  `GET /api/push/key`, and a VAPID keypair generated into the database on first
  run. Alongside it, the client is now an installable progressive web app: a
  manifest with maskable icons, and a service worker that serves the app shell
  offline — cache-first for content-hashed assets, network-first for everything
  else so a deploy is never pinned by a stale `index.html`.
- **Search operators** — `from:`, `in:`, `before:`, `after:` and `has:link` /
  `has:file` / `has:image`, parsed out of the query in `tensorchat_core::query`. Date
  bounds become id bounds, since ids are time-sortable, so they need no
  timestamp column and no second index. A query of operators alone is a valid
  search answered newest-first. An unrecognized `key:value` stays free text, so
  a typo searches rather than silently widening the query, and an operator that
  names something nonexistent returns nothing rather than being dropped.
- **Theme toggle** — System / Light / Dark in Preferences, replacing a light
  theme that could only follow `prefers-color-scheme`. "System" remains the
  default and keeps tracking the OS live. The light palette moved from a media
  query to `:root[data-theme='light']` so it exists exactly once; a small
  classic script stamps the resolved theme before the first paint, since the
  main bundle is a module and therefore deferred, and the CSP has no
  `unsafe-inline` to put it in the page.
- **Inline message editing** — editing happens in the message row instead of a
  native `prompt()`, with Enter to save, Shift+Enter for a newline and Escape
  to cancel. Up-arrow on an empty composer opens your last message; the hook
  for that existed but had never been connected. The editor survives the
  virtual list recycling its row mid-edit, caret position included.
- **Drafts survive a reload** — unsent text moved from an in-memory `Map` to
  `localStorage`. Writes are debounced so typing never blocks on a synchronous
  disk write, and flushed on `pagehide` and on the tab being hidden so a reload
  moments after the last keystroke keeps it. Drafts are pruned by age and
  count, truncated to the server's body limit, and cleared on sign-out.
- **Invite links** — administrators mint links from Preferences → Invite
  people, or `POST /api/admin/invites`. A link may be single-use or multi-use,
  expiring or permanent, and is revocable. `/api/register` accepts an `invite`
  field that admits an account even when `TC_OPEN_REGISTRATION` is false,
  closing the gap where a closed workspace had no way at all to add its second
  person. Recipients open `#/join/{token}`, which pre-checks the link and opens
  the sign-up form. Only a SHA-256 of the token is stored, and the seat is
  claimed inside the same transaction that creates the account, so a single-use
  link cannot admit two people who race each other.
- **Bots and API tokens** — administrators create bot accounts and mint
  long-lived tokens for them. A token authenticates the whole HTTP API as a
  bearer credential, and `POST /api/hooks/{token}` posts for senders that
  cannot set headers. A bot reaches exactly the channels it is a member of.
- **Jump to a message** — search hits, saved items and permalinks open the
  channel positioned on the message, via `?around=` on the history endpoint.
  A "Copy link to message" action produces `#/c/{channel}/{message}` links, and
  a bar offers "Jump to latest" while viewing a historical window.
- **Administrators** — the first account to register becomes one, and can
  promote others or deactivate accounts via `PATCH /api/admin/users/{id}` and a
  "Manage people" dialog. `User.deactivated` existed in the model but nothing
  had ever set it.
- **Emoji picker and `:shortcode:` completion** — a searchable picker on the
  composer and the message hover bar, and inline completion sharing the
  mention popover. Shortcodes expand on send, so what is stored is the emoji.
- **Desktop notifications** — mentions and direct messages raise a browser
  notification when you are not already looking at them. Opt-in from
  Preferences. Client-side only; real Web Push is still not implemented.
- **Channel mute** — `POST /api/channels/{id}/mute`, with a toggle in the
  channel header. A muted channel loses its unread emphasis but keeps its
  mention badge, and the counts are still reported truthfully rather than
  zeroed.
- **Saved messages** — a private, cross-channel collection reachable from the
  sidebar. `POST /api/messages/{id}/save`, `GET /api/saved`, and a per-user
  `saved` frame so a save syncs across tabs.
- **Pinned messages** — any member can pin a message to a channel; pins appear
  in a side pane, are marked in the main scroll, and are capped at 100 per
  channel. `POST /api/messages/{id}/pin`, `GET /api/channels/{id}/pins`, and a
  `pin` broadcast frame.
- **Schema migrations** — the database is upgraded in place on startup. Fresh
  installs get `schema.sql`; existing ones replay the numbered migrations newer
  than their `user_version`. A test asserts the two paths produce an identical
  schema.
- **Password changes** — `POST /api/me/password`, which re-verifies the current
  password and then revokes every *other* session, plus
  `DELETE /api/me/sessions` to do the revocation on its own. Both are exposed in
  Preferences. There was previously no way to change a password at all.
- **Channel invitations** — `POST /api/channels/{id}/members` and
  `DELETE /api/channels/{id}/members/{user}`, with an "Add people" control in the
  member pane. Private channels previously had no way in after creation: `join`
  refuses them by design, and nothing else could grant membership.

### Changed

- `invites.created_by` is now nullable (schema 9). It assumed every invite was
  minted by an administrator through the API, which the bootstrap case cannot
  satisfy — a fresh closed workspace has no accounts at all. The alternatives
  were a synthetic "system" account that every mention search and member list
  would have to hide, or an id that violates the foreign key.

### Fixed

- A reaction, pin, or edit landing on a message that was already on screen now
  repaints it. `VirtualList` deliberately leaves an already-mounted,
  still-visible row alone so that scrolling does not rebuild stationary rows —
  correct for scrolling, but it also meant a change to a visible message's
  *content* was invisible until the reader happened to scroll it out of the
  window and back. A new `VirtualList.refresh()` repaints mounted rows, and the
  message list calls it whenever the data behind them changes.
- Login no longer succeeds for a deactivated account. It previously issued a
  token that then failed on every authenticated request, because `session_user`
  filtered deactivated accounts but `user_for_login` did not.
- Leaving or being removed from a channel now drops the hub subscription as well
  as the membership row. A subscription used to outlive membership until the
  socket reconnected, so a departed member kept receiving a private channel's
  messages for as long as their tab stayed open.

## [0.1.0] — 2026-07-29

Initial release.

### Added

- **Messaging** — public and private channels, direct and group messages,
  threaded replies, edit and delete, emoji reactions, file attachments, and
  `@mentions` with `@here` / `@channel`.
- **Realtime** — WebSocket delivery of messages, reactions, typing indicators,
  presence, membership changes, and read state, with automatic reconnection and
  resynchronization. Each event is serialized once and shared across all
  recipient sockets.
- **Reading** — unread and mention badges, a per-channel read cursor
  synchronized across devices, and FTS5 full-text search with highlighted
  snippets, scoped to the channels the caller belongs to.
- **Client** — virtual-scrolled history, optimistic sending, per-channel drafts,
  mention autocomplete, drag-drop and paste-to-upload, light and dark themes,
  and keyboard navigation. No framework and no runtime dependencies.
- **Operations** — a single binary serving its own frontend, a single SQLite
  file created on first run, and configuration entirely through environment
  variables.
- **Security** — Argon2id password hashing, opaque session tokens stored only as
  SHA-256 digests, membership checks inside the same transaction as every write,
  a Content-Security-Policy with no `unsafe-inline`, and a content-type
  allowlist for uploads.

[Unreleased]: https://github.com/tensorspace-ai/tensorchat/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/tensorspace-ai/tensorchat/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/tensorspace-ai/tensorchat/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/tensorspace-ai/tensorchat/releases/tag/v0.1.0
