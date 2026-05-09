CREATE TABLE IF NOT EXISTS movies (
    id                   TEXT PRIMARY KEY NOT NULL,
    external_metadata_id TEXT UNIQUE,
    title                TEXT NOT NULL,
    release_year         BIGINT NOT NULL,
    director             TEXT,
    poster_path          TEXT
);

CREATE INDEX IF NOT EXISTS idx_movies_title_year ON movies (title, release_year);

CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY NOT NULL,
    email         TEXT UNIQUE NOT NULL,
    username      TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL,
    role          TEXT NOT NULL DEFAULT 'standard'
);

CREATE TABLE IF NOT EXISTS reviews (
    id               TEXT PRIMARY KEY NOT NULL,
    movie_id         TEXT NOT NULL REFERENCES movies(id),
    user_id          TEXT NOT NULL,
    rating           BIGINT NOT NULL,
    comment          TEXT,
    watched_at       TIMESTAMPTZ NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    remote_actor_url TEXT,
    ap_id            TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_ap_id ON reviews (ap_id) WHERE ap_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_reviews_movie_id ON reviews (movie_id);
CREATE INDEX IF NOT EXISTS idx_reviews_watched_at ON reviews (watched_at);

CREATE TABLE IF NOT EXISTS ap_followers (
    local_user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remote_actor_url   TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'pending',
    follow_activity_id TEXT,
    created_at         TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (local_user_id, remote_actor_url)
);

CREATE TABLE IF NOT EXISTS ap_following (
    local_user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remote_actor_url   TEXT NOT NULL,
    follow_activity_id TEXT,
    status             TEXT NOT NULL DEFAULT 'pending',
    created_at         TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (local_user_id, remote_actor_url)
);

CREATE TABLE IF NOT EXISTS ap_remote_actors (
    url              TEXT PRIMARY KEY,
    handle           TEXT NOT NULL,
    inbox_url        TEXT NOT NULL,
    shared_inbox_url TEXT,
    display_name     TEXT,
    fetched_at       TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS ap_local_actors (
    user_id     TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key  TEXT NOT NULL,
    private_key TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL
);
