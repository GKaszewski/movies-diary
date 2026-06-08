CREATE TABLE IF NOT EXISTS goals (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year         INTEGER NOT NULL,
    target_count INTEGER NOT NULL,
    goal_type    TEXT NOT NULL DEFAULT 'movies',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
    UNIQUE(user_id, year)
);
CREATE INDEX IF NOT EXISTS idx_goals_user ON goals(user_id);

CREATE TABLE IF NOT EXISTS user_settings (
    user_id        TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    federate_goals INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS remote_goals (
    ap_id         TEXT PRIMARY KEY NOT NULL,
    actor_url     TEXT NOT NULL,
    year          INTEGER NOT NULL,
    target_count  INTEGER NOT NULL,
    current_count INTEGER NOT NULL DEFAULT 0,
    received_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_remote_goals_actor ON remote_goals(actor_url);
