CREATE TABLE IF NOT EXISTS event_queue (
    id              BIGSERIAL    PRIMARY KEY,
    event_type      TEXT         NOT NULL,
    payload         TEXT         NOT NULL,
    status          TEXT         NOT NULL DEFAULT 'pending',
    attempts        INTEGER      NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    next_attempt_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_error      TEXT
);

CREATE INDEX IF NOT EXISTS idx_event_queue_poll
    ON event_queue (status, next_attempt_at);
