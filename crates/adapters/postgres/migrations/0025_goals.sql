CREATE TABLE IF NOT EXISTS goals (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year         BIGINT NOT NULL,
    target_count BIGINT NOT NULL,
    goal_type    TEXT NOT NULL DEFAULT 'movies',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, year)
);
CREATE INDEX IF NOT EXISTS idx_goals_user ON goals(user_id);

CREATE TABLE IF NOT EXISTS user_settings (
    user_id        TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    federate_goals BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS remote_goals (
    ap_id         TEXT PRIMARY KEY NOT NULL,
    actor_url     TEXT NOT NULL,
    year          BIGINT NOT NULL,
    target_count  BIGINT NOT NULL,
    current_count BIGINT NOT NULL DEFAULT 0,
    received_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_remote_goals_actor ON remote_goals(actor_url);
