use crate::{
    models::{AnnotatedRow, FieldMapping, ParsedFile},
    value_objects::{ImportSessionId, UserId},
};
use chrono::NaiveDateTime;

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

pub struct PersistedImportSession {
    pub id: ImportSessionId,
    pub user_id: UserId,
    pub parsed_file: Option<ParsedFile>,
    pub field_mappings: Option<Vec<FieldMapping>>,
    pub row_results: Option<Vec<AnnotatedRow>>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

impl ImportSession {
    pub fn new(user_id: UserId) -> Self {
        let created_at = chrono::Utc::now().naive_utc();
        let expires_at = created_at + chrono::Duration::hours(24);
        Self {
            id: ImportSessionId::generate(),
            user_id,
            parsed_file: None,
            field_mappings: None,
            row_results: None,
            created_at,
            expires_at,
        }
    }

    pub fn from_persistence(p: PersistedImportSession) -> Self {
        Self {
            id: p.id,
            user_id: p.user_id,
            parsed_file: p.parsed_file,
            field_mappings: p.field_mappings,
            row_results: p.row_results,
            created_at: p.created_at,
            expires_at: p.expires_at,
        }
    }
}
