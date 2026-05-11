CREATE TABLE blocked_domains (
    domain     TEXT PRIMARY KEY,
    reason     TEXT,
    blocked_at TEXT NOT NULL
);
