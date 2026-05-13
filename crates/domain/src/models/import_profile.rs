use crate::{
    models::FieldMapping,
    value_objects::{ImportProfileId, UserId},
};
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct ImportProfile {
    pub id: ImportProfileId,
    pub user_id: UserId,
    pub name: String,
    pub field_mappings: Vec<FieldMapping>,
    pub created_at: NaiveDateTime,
}

impl ImportProfile {
    pub fn new(
        id: ImportProfileId,
        user_id: UserId,
        name: String,
        field_mappings: Vec<FieldMapping>,
        created_at: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            name,
            field_mappings,
            created_at,
        }
    }
}
