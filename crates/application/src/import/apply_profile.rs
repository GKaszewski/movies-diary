use std::sync::Arc;

use crate::import::commands::ApplyImportProfileCommand;
use domain::{
    errors::DomainError,
    ports::{ImportProfileRepository, ImportSessionRepository},
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

/// Copies the profile's field_mappings onto the session. Caller must then invoke
/// apply_import_mapping to regenerate row_results with the new mappings.
pub async fn execute(
    import_profile: Arc<dyn ImportProfileRepository>,
    import_session: Arc<dyn ImportSessionRepository>,
    cmd: ApplyImportProfileCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    let profile = import_profile
        .get(&profile_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    let mut session = import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;
    session.field_mappings = Some(profile.field_mappings);
    session.row_results = None;
    import_session.update(&session).await
}

#[cfg(test)]
#[path = "tests/apply_profile.rs"]
mod tests;
