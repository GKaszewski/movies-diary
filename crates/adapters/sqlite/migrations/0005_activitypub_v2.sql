-- Store the original Follow activity URL so Undo/Reject can reference it correctly
ALTER TABLE ap_following ADD COLUMN follow_activity_id TEXT;

-- Track whether our outbound follow was accepted by the remote server
ALTER TABLE ap_following ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';

-- Store the AP object URL on reviews so DeleteActivity can target by ID
ALTER TABLE reviews ADD COLUMN ap_id TEXT;

-- Partial unique index: ap_id is only set on remote reviews; local reviews have NULL
CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_ap_id ON reviews (ap_id) WHERE ap_id IS NOT NULL;
