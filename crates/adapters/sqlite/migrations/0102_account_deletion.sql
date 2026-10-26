ALTER TABLE users ADD COLUMN deleted_at TEXT;
-- Set once the account's data is purged; the stripped row (and the signing
-- keypair) stays until the tombstone cleanup removes it.
ALTER TABLE users ADD COLUMN purged_at TEXT;

CREATE TABLE retired_usernames (
    username TEXT PRIMARY KEY COLLATE NOCASE,
    retired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
