ALTER TABLE users ADD COLUMN IF NOT EXISTS banner_path TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS also_known_as TEXT;

CREATE TABLE IF NOT EXISTS user_profile_fields (
    id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_profile_fields_user_id
    ON user_profile_fields(user_id);
