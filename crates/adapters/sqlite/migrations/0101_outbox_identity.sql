-- Outbox identity: every queued event carries a stable id; handlers claim
-- (event_id, handler) pairs so redeliveries are skipped and concurrent
-- deliveries never run the same handler twice.
ALTER TABLE event_queue ADD COLUMN event_id TEXT;

-- When the current claim started; a claim older than the lease is reclaimable.
ALTER TABLE event_queue ADD COLUMN claimed_at TEXT;

-- Legacy rows get a random v4-shaped uuid. The subquery references the row id
-- so it is re-evaluated per row rather than cached.
UPDATE event_queue SET event_id = (
    SELECT lower(
        substr(h, 1, 8) || '-' || substr(h, 9, 4) || '-4' || substr(h, 14, 3) || '-'
        || substr('89ab', (abs(random()) % 4) + 1, 1) || substr(h, 18, 3) || '-' || substr(h, 21, 12)
    )
    FROM (SELECT hex(randomblob(16)) AS h, event_queue.id AS row_id)
);

CREATE UNIQUE INDEX idx_event_queue_event_id
    ON event_queue (event_id) WHERE event_id IS NOT NULL;

CREATE TABLE processed_events (
    event_id     TEXT NOT NULL,
    handler      TEXT NOT NULL,
    status       TEXT NOT NULL CHECK (status IN ('in_progress', 'done')),
    started_at   TEXT,
    processed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (event_id, handler)
);
