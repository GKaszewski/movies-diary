CREATE TABLE IF NOT EXISTS wrap_up_records (
    id              TEXT PRIMARY KEY NOT NULL,
    user_id         TEXT,
    start_date      DATE NOT NULL,
    end_date        DATE NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    report_json     TEXT,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_wrap_up_user ON wrap_up_records (user_id, start_date, end_date);
