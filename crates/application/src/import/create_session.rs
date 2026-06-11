use std::sync::Arc;

use chrono::Utc;
use domain::{
    errors::DomainError,
    models::ImportSession,
    ports::{DocumentParser, ImportSessionRepository},
    value_objects::{ImportSessionId, UserId},
};

use crate::import::commands::CreateImportSessionCommand;

pub struct CreateSessionResult {
    pub session_id: ImportSessionId,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

pub async fn execute(
    import_session: Arc<dyn ImportSessionRepository>,
    document_parser: Arc<dyn DocumentParser>,
    cmd: CreateImportSessionCommand,
) -> Result<CreateSessionResult, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    import_session
        .delete_expired_for_user(&user_id)
        .await?;

    let parsed = document_parser
        .parse(&cmd.bytes, cmd.format)
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

    let sample_rows = parsed.rows.iter().take(5).cloned().collect();
    let columns = parsed.columns.clone();

    let now = Utc::now().naive_utc();
    let mut session = ImportSession::new(ImportSessionId::generate(), user_id, now);
    let session_id = session.id.clone();
    session.parsed_file = Some(parsed);

    import_session.create(&session).await?;

    Ok(CreateSessionResult {
        session_id,
        columns,
        sample_rows,
    })
}

#[cfg(test)]
#[path = "tests/create_session.rs"]
mod tests;
