use chrono::Utc;
use domain::{errors::DomainError, models::ImportSession, value_objects::{ImportSessionId, UserId}};
use importer::{ImportError, ParsedFile};

use crate::{commands::{CreateImportSessionCommand, FileFormat}, context::AppContext};

pub struct CreateSessionResult {
    pub session_id: ImportSessionId,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

pub async fn execute(ctx: &AppContext, cmd: CreateImportSessionCommand) -> Result<CreateSessionResult, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    ctx.import_session_repository.delete_expired_for_user(&user_id).await?;

    let parsed = parse(cmd.bytes, cmd.format).map_err(|e| DomainError::ValidationError(e.to_string()))?;
    let sample_rows = parsed.rows.iter().take(5).cloned().collect();
    let columns = parsed.columns.clone();

    let parsed_data = serde_json::to_string(&parsed)
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

    let now = Utc::now().naive_utc();
    let session = ImportSession::new(ImportSessionId::generate(), user_id, parsed_data, now);
    let session_id = session.id.clone();

    ctx.import_session_repository.create(&session).await?;

    Ok(CreateSessionResult { session_id, columns, sample_rows })
}

fn parse(bytes: Vec<u8>, format: FileFormat) -> Result<ParsedFile, ImportError> {
    match format {
        FileFormat::Csv => importer::parse_csv(&bytes),
        FileFormat::Json => importer::parse_json(&bytes),
        FileFormat::Xlsx => {
            #[cfg(feature = "xlsx")]
            { importer::parse_xlsx(&bytes) }
            #[cfg(not(feature = "xlsx"))]
            { Err(ImportError::Xlsx("XLSX support not compiled in".into())) }
        }
    }
}
