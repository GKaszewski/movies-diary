CREATE TABLE IF NOT EXISTS webhook_tokens (
    id            TEXT PRIMARY KEY NOT NULL,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash    TEXT NOT NULL,
    provider      TEXT NOT NULL,
    label         TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_webhook_tokens_hash ON webhook_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_webhook_tokens_user ON webhook_tokens(user_id);

CREATE TABLE IF NOT EXISTS watch_events (
    id                    TEXT PRIMARY KEY NOT NULL,
    user_id               TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    movie_id              TEXT REFERENCES movies(id) ON DELETE SET NULL,
    title                 TEXT NOT NULL,
    year                  INTEGER,
    external_metadata_id  TEXT,
    source                TEXT NOT NULL,
    watched_at            TIMESTAMPTZ NOT NULL,
    status                TEXT NOT NULL DEFAULT 'pending',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_watch_events_user_status ON watch_events(user_id, status);
CREATE INDEX IF NOT EXISTS idx_watch_events_dedup ON watch_events(user_id, external_metadata_id, created_at);
