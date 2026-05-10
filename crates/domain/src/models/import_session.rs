use chrono::NaiveDateTime;
use crate::value_objects::{ImportSessionId, UserId};

#[derive(Debug, Clone)]
pub struct ImportSession {
    pub id: ImportSessionId,
    pub user_id: UserId,
    pub parsed_data: String,
    pub field_mappings: Option<String>,
    pub row_results: Option<String>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

impl ImportSession {
    pub fn new(id: ImportSessionId, user_id: UserId, parsed_data: String, created_at: NaiveDateTime) -> Self {
        let expires_at = created_at + chrono::Duration::hours(24);
        Self { id, user_id, parsed_data, field_mappings: None, row_results: None, created_at, expires_at }
    }
}
