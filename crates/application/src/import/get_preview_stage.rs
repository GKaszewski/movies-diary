use domain::{
    errors::DomainError,
    models::AnnotatedRow,
    value_objects::{ImportSessionId, UserId},
};
use uuid::Uuid;

use crate::import::deps::GetPreviewStageDeps;

pub struct PreviewRows {
    pub columns: Vec<String>,
    pub rows: Vec<AnnotatedRow>,
}

pub enum PreviewStage {
    Ready(PreviewRows),
    NotYetMapped,
}

pub async fn execute(
    deps: &GetPreviewStageDeps,
    session_id: ImportSessionId,
    user_id: Uuid,
) -> Result<PreviewStage, DomainError> {
    let user_id = UserId::from_uuid(user_id);
    let session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("session not found".into()))?;

    let Some(rows) = session.row_results else {
        return Ok(PreviewStage::NotYetMapped);
    };

    let columns = session.parsed_file.map(|p| p.columns).unwrap_or_default();

    Ok(PreviewStage::Ready(PreviewRows { columns, rows }))
}

#[cfg(test)]
#[path = "tests/get_preview_stage.rs"]
mod tests;
