use domain::{
    errors::DomainError,
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

use super::{commands::ApplyImportProfileCommand, deps::ApplyProfileDeps};

pub async fn execute(
    deps: &ApplyProfileDeps,
    cmd: ApplyImportProfileCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    let profile = deps
        .import_profile
        .get(&profile_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    let mut session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;
    session.field_mappings = Some(profile.field_mappings);
    session.row_results = None;
    deps.import_session.update(&session).await
}

#[cfg(test)]
#[path = "tests/apply_profile.rs"]
mod tests;
