//! The mapping-page stage gate: a session must exist and have a `parsed_file`
//! before its columns/sample rows can be shown for field mapping. Absorbs
//! `handlers/import.rs::get_mapping_page`'s two early-return checks (session
//! missing, `parsed_file` absent) — both collapse to `NotFound` here since the
//! handler redirected to the same place (`/import`) for either.

use domain::{errors::DomainError, value_objects::ImportSessionId};
use uuid::Uuid;

use crate::import::deps::GetMappingStageDeps;

/// Cap on sample rows shown on the mapping page — was a bare `.take(5)` in the
/// handler.
pub const SAMPLE_ROW_LIMIT: usize = 5;

pub struct MappingStage {
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

pub async fn execute(
    deps: &GetMappingStageDeps,
    session_id: ImportSessionId,
    user_id: Uuid,
) -> Result<MappingStage, DomainError> {
    let user_id = domain::value_objects::UserId::from_uuid(user_id);
    let session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;

    let parsed = session
        .parsed_file
        .ok_or_else(|| DomainError::NotFound("import session has no parsed file".into()))?;

    let sample_rows = parsed.rows.into_iter().take(SAMPLE_ROW_LIMIT).collect();

    Ok(MappingStage {
        columns: parsed.columns,
        sample_rows,
    })
}

#[cfg(test)]
#[path = "tests/get_mapping_stage.rs"]
mod tests;
