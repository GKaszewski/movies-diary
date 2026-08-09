//! Backs `handlers/import.rs::api_get_session` — a plain state query, not a
//! redirect-driving gate (the API has nothing to redirect to; a missing
//! session is just a 404).

use domain::{
    errors::DomainError,
    value_objects::{ImportSessionId, UserId},
};
use uuid::Uuid;

use crate::import::deps::GetSessionStateDeps;

pub struct SessionState {
    pub columns: Vec<String>,
    pub has_mappings: bool,
    pub row_count: usize,
}

pub async fn execute(
    deps: &GetSessionStateDeps,
    session_id: ImportSessionId,
    user_id: Uuid,
) -> Result<SessionState, DomainError> {
    let user_id = UserId::from_uuid(user_id);
    let session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("session not found".into()))?;

    let parsed = session.parsed_file.unwrap_or_default();
    let row_count = parsed.rows.len();

    Ok(SessionState {
        columns: parsed.columns,
        has_mappings: session.field_mappings.is_some(),
        row_count,
    })
}

#[cfg(test)]
#[path = "tests/get_session_state.rs"]
mod tests;
