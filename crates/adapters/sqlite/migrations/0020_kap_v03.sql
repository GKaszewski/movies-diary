-- k-ap 0.3: new RemoteActor fields + activity dedup table

ALTER TABLE ap_remote_actors ADD COLUMN bio TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN banner_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN followers_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN following_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN also_known_as TEXT; -- JSON array

CREATE TABLE IF NOT EXISTS ap_activities (
    id TEXT PRIMARY KEY,
    processed_at TEXT NOT NULL
);
