use chrono::Utc;
use domain::{
    errors::DomainError,
    models::ImportProfile,
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

use super::{commands::SaveImportProfileCommand, deps::SaveProfileDeps};

pub async fn execute(
    deps: &SaveProfileDeps,
    cmd: SaveImportProfileCommand,
) -> Result<ImportProfileId, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);

    let session = deps
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
    deps.import_profile.save(&profile).await?;
    Ok(id)
}

#[cfg(test)]
#[path = "tests/save_profile.rs"]
mod tests;
