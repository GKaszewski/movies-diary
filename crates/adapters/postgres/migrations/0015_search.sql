CREATE TABLE IF NOT EXISTS persons (
    id             TEXT PRIMARY KEY,
    external_id    TEXT NOT NULL UNIQUE,
    tmdb_person_id BIGINT UNIQUE,
    name           TEXT NOT NULL,
    known_for_department TEXT,
    profile_path   TEXT
);

CREATE INDEX IF NOT EXISTS idx_persons_external ON persons (external_id);
CREATE INDEX IF NOT EXISTS idx_persons_tmdb_id  ON persons (tmdb_person_id);

-- tsvector-based search for movies (equivalent of SQLite FTS5)
CREATE TABLE IF NOT EXISTS movies_search (
    movie_id    TEXT PRIMARY KEY REFERENCES movies(id) ON DELETE CASCADE,
    fts         TSVECTOR NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_movies_search_fts ON movies_search USING GIN(fts);

-- tsvector-based search for people
CREATE TABLE IF NOT EXISTS people_search (
    person_id   TEXT PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
    fts         TSVECTOR NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_people_search_fts ON people_search USING GIN(fts);
