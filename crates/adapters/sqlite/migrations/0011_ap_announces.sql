CREATE TABLE ap_announces (
    id           TEXT PRIMARY KEY,
    object_url   TEXT NOT NULL,
    actor_url    TEXT NOT NULL,
    announced_at TEXT NOT NULL
);

CREATE INDEX idx_ap_announces_object ON ap_announces (object_url);
