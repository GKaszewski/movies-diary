-- Persons table. tmdb_person_id is stored for efficient joins with existing
-- movie_cast and movie_crew tables (which use tmdb_person_id as their person key).
CREATE TABLE IF NOT EXISTS persons (
    id             TEXT PRIMARY KEY,        -- UUID (PersonId)
    external_id    TEXT NOT NULL UNIQUE,    -- "tmdb:12345"
    tmdb_person_id INTEGER UNIQUE,          -- parsed from external_id for fast joins
    name           TEXT NOT NULL,
    known_for_department TEXT,
    profile_path   TEXT
);

CREATE INDEX IF NOT EXISTS idx_persons_external ON persons (external_id);
CREATE INDEX IF NOT EXISTS idx_persons_tmdb_id  ON persons (tmdb_person_id);

-- FTS5 full-text search table for movies.
-- movie_id is UNINDEXED (stored but not tokenised for text search).
-- release_year and language are UNINDEXED (used only for structured filters).
CREATE VIRTUAL TABLE IF NOT EXISTS movies_fts USING fts5(
    movie_id     UNINDEXED,
    title,
    director,
    overview,
    genres,
    keywords,
    cast_names,
    crew_names,
    release_year UNINDEXED,
    language     UNINDEXED
);

-- FTS5 full-text search table for people.
CREATE VIRTUAL TABLE IF NOT EXISTS people_fts USING fts5(
    person_id            UNINDEXED,
    name,
    known_for_department UNINDEXED
);
