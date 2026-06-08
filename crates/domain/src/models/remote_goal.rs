use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct RemoteGoalEntry {
    pub ap_id: String,
    pub actor_url: String,
    pub year: u16,
    pub target_count: u32,
    pub current_count: u32,
    pub received_at: DateTime<Utc>,
}
