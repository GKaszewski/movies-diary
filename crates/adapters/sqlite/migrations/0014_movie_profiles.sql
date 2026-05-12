CREATE TABLE IF NOT EXISTS movie_profiles (
    movie_id          TEXT PRIMARY KEY NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    tmdb_id           INTEGER NOT NULL,
    imdb_id           TEXT,
    overview          TEXT,
    tagline           TEXT,
    runtime_minutes   INTEGER,
    budget_usd        INTEGER,
    revenue_usd       INTEGER,
    vote_average      REAL,
    vote_count        INTEGER,
    original_language TEXT,
    collection_name   TEXT,
    enriched_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS movie_genres (
    movie_id  TEXT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    tmdb_id   INTEGER NOT NULL,
    name      TEXT NOT NULL,
    PRIMARY KEY (movie_id, tmdb_id)
);

CREATE TABLE IF NOT EXISTS movie_keywords (
    movie_id  TEXT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    tmdb_id   INTEGER NOT NULL,
    name      TEXT NOT NULL,
    PRIMARY KEY (movie_id, tmdb_id)
);

CREATE TABLE IF NOT EXISTS movie_cast (
    movie_id        TEXT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    tmdb_person_id  INTEGER NOT NULL,
    name            TEXT NOT NULL,
    character       TEXT NOT NULL,
    billing_order   INTEGER NOT NULL,
    profile_path    TEXT,
    PRIMARY KEY (movie_id, tmdb_person_id)
);

CREATE TABLE IF NOT EXISTS movie_crew (
    movie_id        TEXT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    tmdb_person_id  INTEGER NOT NULL,
    name            TEXT NOT NULL,
    job             TEXT NOT NULL,
    department      TEXT NOT NULL,
    profile_path    TEXT,
    PRIMARY KEY (movie_id, tmdb_person_id, job)
);

CREATE INDEX IF NOT EXISTS idx_movie_cast_person ON movie_cast (tmdb_person_id);
CREATE INDEX IF NOT EXISTS idx_movie_crew_person ON movie_crew (tmdb_person_id);
CREATE INDEX IF NOT EXISTS idx_movie_genres_name ON movie_genres (name);
CREATE INDEX IF NOT EXISTS idx_movie_keywords_name ON movie_keywords (name);
