CREATE TABLE IF NOT EXISTS import_sessions (
    id            TEXT PRIMARY KEY NOT NULL,
    user_id       TEXT NOT NULL,
    parsed_data   TEXT NOT NULL,
    field_mappings TEXT,
    row_results   TEXT,
    created_at    TEXT NOT NULL,
    expires_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_profiles (
    id             TEXT PRIMARY KEY NOT NULL,
    user_id        TEXT NOT NULL,
    name           TEXT NOT NULL,
    field_mappings TEXT NOT NULL,
    created_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_import_sessions_user_id ON import_sessions (user_id);
CREATE INDEX IF NOT EXISTS idx_import_sessions_expires_at ON import_sessions (expires_at);
CREATE INDEX IF NOT EXISTS idx_import_profiles_user_id ON import_profiles (user_id);
