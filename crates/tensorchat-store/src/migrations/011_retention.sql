-- Retention markers, attachment age, and a retryable file-deletion queue.
ALTER TABLE channels ADD COLUMN retention_at INTEGER;
ALTER TABLE channels ADD COLUMN retention_before INTEGER;
ALTER TABLE attachments ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;

CREATE TABLE blob_deletions (
    path TEXT PRIMARY KEY
) STRICT, WITHOUT ROWID;
