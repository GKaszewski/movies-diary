use chrono::Utc;
use domain::{
    errors::DomainError,
    models::ImportSession,
    value_objects::{ImportSessionId, UserId},
};

use crate::{context::AppContext, import::commands::CreateImportSessionCommand};

pub struct CreateSessionResult {
    pub session_id: ImportSessionId,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

pub async fn execute(
    ctx: &AppContext,
    cmd: CreateImportSessionCommand,
) -> Result<CreateSessionResult, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    ctx.repos
        .import_session
        .delete_expired_for_user(&user_id)
        .await?;

    let parsed = ctx
        .services
        .document_parser
        .parse(&cmd.bytes, cmd.format)
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

    let sample_rows = parsed.rows.iter().take(5).cloned().collect();
    let columns = parsed.columns.clone();

    let now = Utc::now().naive_utc();
    let mut session = ImportSession::new(ImportSessionId::generate(), user_id, now);
    let session_id = session.id.clone();
    session.parsed_file = Some(parsed);

    ctx.repos.import_session.create(&session).await?;

    Ok(CreateSessionResult {
        session_id,
        columns,
        sample_rows,
    })
}
