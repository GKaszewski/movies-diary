-- Recreate users table with username column
CREATE TABLE users_new (
    id           TEXT PRIMARY KEY,
    email        TEXT NOT NULL UNIQUE,
    username     TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

-- Derive username from email local part, sanitising common invalid chars.
-- REPLACE chains handle the most common email chars. The NOT NULL UNIQUE
-- constraint will surface any remaining collision (rare for personal instances).
INSERT INTO users_new (id, email, username, password_hash, created_at)
SELECT
    id,
    email,
    CASE
        WHEN LENGTH(REPLACE(REPLACE(REPLACE(REPLACE(
            LOWER(SUBSTR(email, 1, INSTR(email, '@') - 1)),
            '.', '_'), '+', '_'), '-', '-'), ' ', '_')) < 2
        THEN REPLACE(REPLACE(REPLACE(REPLACE(
            LOWER(SUBSTR(email, 1, INSTR(email, '@') - 1)),
            '.', '_'), '+', '_'), '-', '-'), ' ', '_') || '_x'
        ELSE REPLACE(REPLACE(REPLACE(REPLACE(
            LOWER(SUBSTR(email, 1, INSTR(email, '@') - 1)),
            '.', '_'), '+', '_'), '-', '-'), ' ', '_')
    END,
    password_hash,
    created_at
FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
