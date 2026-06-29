use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::value_objects::UserId;

pub struct GeneratedToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct RefreshSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
