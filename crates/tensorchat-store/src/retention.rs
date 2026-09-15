//! Time-based deletion of conversations, messages, and staged uploads.

use rusqlite::{TransactionBehavior, params};
use tensorchat_core::Id;

use crate::{Result, Store, from_sql, to_sql};

/// Work completed by one retention pass. Blob paths remain queued until the
/// server has removed their files too.
#[derive(Debug, Default)]
pub struct RetentionPurge {
    pub channels: Vec<Id>,
    pub pruned_channels: Vec<Id>,
}

impl Store {
    /// Remove data older than `cutoff` in one IMMEDIATE transaction.
    ///
    /// A channel with no activity in the period is removed outright. Active
    /// channels keep newer messages and gain one updated retention marker.
    pub fn purge_retention(
        &self,
        cutoff: Id,
        cutoff_ms: u64,
        now_ms: u64,
    ) -> Result<RetentionPurge> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let channels: Vec<Id> = {
            let mut stmt = tx.prepare_cached(
                "SELECT id FROM channels WHERE \
                 (last_message = 0 AND created_at < ?) OR last_message < ?",
            )?;
            stmt.query_map(params![cutoff_ms as i64, to_sql(cutoff)], |r| {
                Ok(from_sql(r.get(0)?))
            })?
            .collect::<rusqlite::Result<_>>()?
        };

        let pruned_channels: Vec<Id> = {
            let mut stmt = tx.prepare_cached(
                "SELECT DISTINCT channel_id FROM messages WHERE id < ? \
                 AND channel_id NOT IN (SELECT id FROM channels WHERE \
                   (last_message = 0 AND created_at < ?) OR last_message < ?)",
            )?;
            stmt.query_map(
                params![to_sql(cutoff), cutoff_ms as i64, to_sql(cutoff)],
                |r| Ok(from_sql(r.get(0)?)),
            )?
            .collect::<rusqlite::Result<_>>()?
        };

        // Queue before foreign-key cascades remove attachment rows. A failed
        // unlink is harmless: the queue keeps it for the next pass.
        tx.execute(
            "INSERT OR IGNORE INTO blob_deletions(path) \
             SELECT DISTINCT path FROM attachments WHERE \
               message_id IN (SELECT id FROM messages WHERE id < ?) \
               OR (message_id IS NULL AND created_at < ?)",
            params![to_sql(cutoff), cutoff_ms as i64],
        )?;
        tx.execute(
            "DELETE FROM mentions WHERE message_id IN (SELECT id FROM messages WHERE id < ?)",
            [to_sql(cutoff)],
        )?;
        tx.execute("DELETE FROM messages WHERE id < ?", [to_sql(cutoff)])?;
        tx.execute(
            "DELETE FROM attachments WHERE message_id IS NULL AND created_at < ?",
            [cutoff_ms as i64],
        )?;

        for channel in &pruned_channels {
            tx.execute(
                "UPDATE channels SET retention_at = ?, retention_before = ? WHERE id = ?",
                params![now_ms as i64, to_sql(cutoff), to_sql(*channel)],
            )?;
        }
        // Rows related to a channel (members, messages, pins, saved, attached
        // blobs) cascade. Mentions have no foreign key, so clear those first.
        tx.execute(
            "DELETE FROM mentions WHERE channel_id IN (SELECT id FROM channels WHERE \
             (last_message = 0 AND created_at < ?) OR last_message < ?)",
            params![cutoff_ms as i64, to_sql(cutoff)],
        )?;
        tx.execute(
            "DELETE FROM channels WHERE (last_message = 0 AND created_at < ?) OR last_message < ?",
            params![cutoff_ms as i64, to_sql(cutoff)],
        )?;
        tx.commit()?;
        Ok(RetentionPurge {
            channels,
            pruned_channels,
        })
    }

    /// A bounded batch makes a pathological directory or antivirus lock unable
    /// to monopolize the periodic maintenance task.
    pub fn pending_blob_deletions(&self, limit: u32) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("SELECT path FROM blob_deletions LIMIT ?")?;
        Ok(stmt
            .query_map([limit.clamp(1, 1000) as i64], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn acknowledge_blob_deletion(&self, path: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM blob_deletions WHERE path = ?", [path])?;
        Ok(())
    }
}
