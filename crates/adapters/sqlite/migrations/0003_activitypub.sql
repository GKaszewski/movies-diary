ALTER TABLE reviews ADD COLUMN remote_actor_url TEXT;
CREATE TABLE ap_followers (
    local_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remote_actor_url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    PRIMARY KEY (local_user_id, remote_actor_url)
);
CREATE TABLE ap_following (
    local_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remote_actor_url TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (local_user_id, remote_actor_url)
);
CREATE TABLE ap_remote_actors (
    url TEXT PRIMARY KEY,
    handle TEXT NOT NULL,
    inbox_url TEXT NOT NULL,
    shared_inbox_url TEXT,
    display_name TEXT,
    fetched_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ap_local_actors (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key TEXT NOT NULL,
    private_key TEXT NOT NULL,
    created_at TEXT NOT NULL
);
