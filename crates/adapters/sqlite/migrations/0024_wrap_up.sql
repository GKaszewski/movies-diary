CREATE TABLE IF NOT EXISTS wrap_up_records (
    id              TEXT PRIMARY KEY NOT NULL,
    user_id         TEXT,
    start_date      TEXT NOT NULL,
    end_date        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    report_json     TEXT,
    error_message   TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
    completed_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_wrap_up_user ON wrap_up_records (user_id, start_date, end_date);
