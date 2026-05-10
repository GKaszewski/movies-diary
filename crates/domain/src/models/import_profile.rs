use chrono::NaiveDateTime;
use crate::value_objects::{ImportProfileId, UserId};

#[derive(Debug, Clone)]
pub struct ImportProfile {
    pub id: ImportProfileId,
    pub user_id: UserId,
    pub name: String,
    pub field_mappings: String,
    pub created_at: NaiveDateTime,
}

impl ImportProfile {
    pub fn new(id: ImportProfileId, user_id: UserId, name: String, field_mappings: String, created_at: NaiveDateTime) -> Self {
        Self { id, user_id, name, field_mappings, created_at }
    }
}
