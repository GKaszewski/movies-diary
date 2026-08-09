//! The preview-page stage gate: a session must have `row_results` (i.e. a
//! mapping has already been applied) before its rows can be previewed. Serves
//! both the HTML preview handler and the API preview handler —
//! `handlers/import.rs::get_preview_page` and `::api_get_preview` — which
//! render/respond to `NotYetMapped` differently (redirect vs. status code); that
//! decision stays in the handlers, not here.

use domain::{
    errors::DomainError,
    models::AnnotatedRow,
    value_objects::{ImportSessionId, UserId},
};
use uuid::Uuid;

use crate::import::deps::GetPreviewStageDeps;

/// The columns and mapped/annotated rows for a session whose mapping has
/// already been applied. `columns` comes from the session's `parsed_file` —
/// the HTML preview template renders it as the table header — while `rows`
/// comes from `row_results`. Not in the brief's `PreviewStage::Ready(Vec<AnnotatedRow>)`
/// sketch: the deleted `get_preview_page` handler code read both
/// `session.parsed_file.columns` and `session.row_results` to render the page,
/// so dropping `columns` here would either blank the preview table's header or
/// force the handler to re-fetch the session itself (forbidden — that's the
/// exact repo call this task removes). See task-2 report for detail.
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
