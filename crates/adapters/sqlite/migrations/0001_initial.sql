CREATE TABLE IF NOT EXISTS movies (
    id                   TEXT PRIMARY KEY NOT NULL,
    external_metadata_id TEXT UNIQUE,
    title                TEXT NOT NULL,
    release_year         INTEGER NOT NULL,
    director             TEXT,
    poster_path          TEXT
);

CREATE INDEX IF NOT EXISTS idx_movies_title_year
    ON movies (title, release_year);

CREATE TABLE IF NOT EXISTS reviews (
    id          TEXT PRIMARY KEY NOT NULL,
    movie_id    TEXT NOT NULL REFERENCES movies(id),
    user_id     TEXT NOT NULL,
    rating      INTEGER NOT NULL,
    comment     TEXT,
    watched_at  TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reviews_movie_id ON reviews (movie_id);
CREATE INDEX IF NOT EXISTS idx_reviews_watched_at ON reviews (watched_at);
