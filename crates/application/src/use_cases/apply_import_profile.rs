use domain::{errors::DomainError, value_objects::{ImportProfileId, ImportSessionId, UserId}};
use crate::{commands::ApplyImportProfileCommand, context::AppContext};

/// Copies the profile's field_mappings onto the session. Caller must then invoke
/// apply_import_mapping to regenerate row_results with the new mappings.
pub async fn execute(ctx: &AppContext, cmd: ApplyImportProfileCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    let profile = ctx.import_profile_repository
        .get(&profile_id, &user_id).await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    let mut session = ctx.import_session_repository
        .get(&session_id, &user_id).await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;
    session.field_mappings = Some(profile.field_mappings);
    session.row_results = None;
    ctx.import_session_repository.update(&session).await
}
