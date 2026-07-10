use domain::{
    errors::DomainError,
    models::ImportSession,
    value_objects::{ImportSessionId, UserId},
};

use super::{commands::CreateImportSessionCommand, deps::CreateSessionDeps};

pub struct CreateSessionResult {
    pub session_id: ImportSessionId,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

pub async fn execute(
    deps: &CreateSessionDeps,
    cmd: CreateImportSessionCommand,
) -> Result<CreateSessionResult, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    deps.import_session
        .delete_expired_for_user(&user_id)
        .await?;

    let parsed = deps
        .document_parser
        .parse(&cmd.bytes, cmd.format)
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

    let sample_rows = parsed.rows.iter().take(5).cloned().collect();
    let columns = parsed.columns.clone();

    let mut session = ImportSession::new(user_id);
    let session_id = session.id.clone();
    session.parsed_file = Some(parsed);

    deps.import_session.create(&session).await?;

    Ok(CreateSessionResult {
        session_id,
        columns,
        sample_rows,
    })
}

#[cfg(test)]
#[path = "tests/create_session.rs"]
mod tests;
