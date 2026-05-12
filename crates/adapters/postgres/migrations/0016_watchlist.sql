CREATE TABLE IF NOT EXISTS watchlist_entries (
    id        TEXT PRIMARY KEY NOT NULL,
    user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    movie_id  TEXT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    added_at  TIMESTAMPTZ NOT NULL,
    UNIQUE(user_id, movie_id)
);

CREATE INDEX IF NOT EXISTS idx_watchlist_user ON watchlist_entries(user_id, added_at DESC);
