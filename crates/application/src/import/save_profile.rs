use crate::{context::AppContext, import::commands::SaveImportProfileCommand};
use chrono::Utc;
use domain::{
    errors::DomainError,
    models::ImportProfile,
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

pub async fn execute(
    ctx: &AppContext,
    cmd: SaveImportProfileCommand,
) -> Result<ImportProfileId, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);

    let session = ctx
        .repos
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;
    let mappings = session.field_mappings.ok_or_else(|| {
        DomainError::ValidationError("no mapping applied to this session yet".into())
    })?;
    let profile = ImportProfile::new(
        ImportProfileId::generate(),
        user_id,
        cmd.name,
        mappings,
        Utc::now().naive_utc(),
    );
    let id = profile.id.clone();
    ctx.repos.import_profile.save(&profile).await?;
    Ok(id)
}
