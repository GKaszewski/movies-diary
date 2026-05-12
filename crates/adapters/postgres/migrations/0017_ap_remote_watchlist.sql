CREATE TABLE IF NOT EXISTS ap_remote_watchlist_entries (
    ap_id        TEXT PRIMARY KEY NOT NULL,
    actor_url    TEXT NOT NULL,
    movie_title  TEXT NOT NULL,
    release_year INTEGER NOT NULL,
    external_metadata_id TEXT,
    poster_url   TEXT,
    added_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_remote_watchlist_actor
    ON ap_remote_watchlist_entries(actor_url);
