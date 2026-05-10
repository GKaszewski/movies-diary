use chrono::NaiveDateTime;
use crate::{
    models::{AnnotatedRow, FieldMapping, ParsedFile},
    value_objects::{ImportSessionId, UserId},
};

#[derive(Debug, Clone)]
pub struct ImportSession {
    pub id: ImportSessionId,
    pub user_id: UserId,
    pub parsed_file: Option<ParsedFile>,
    pub field_mappings: Option<Vec<FieldMapping>>,
    pub row_results: Option<Vec<AnnotatedRow>>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

impl ImportSession {
    pub fn new(id: ImportSessionId, user_id: UserId, created_at: NaiveDateTime) -> Self {
        let expires_at = created_at + chrono::Duration::hours(24);
        Self {
            id,
            user_id,
            parsed_file: None,
            field_mappings: None,
            row_results: None,
            created_at,
            expires_at,
        }
    }
}
