-- k-ap 0.3: new RemoteActor fields + activity dedup table

ALTER TABLE ap_remote_actors ADD COLUMN IF NOT EXISTS bio TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN IF NOT EXISTS banner_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN IF NOT EXISTS followers_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN IF NOT EXISTS following_url TEXT;
ALTER TABLE ap_remote_actors ADD COLUMN IF NOT EXISTS also_known_as TEXT; -- JSON array

CREATE TABLE IF NOT EXISTS ap_activities (
    id TEXT PRIMARY KEY,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
