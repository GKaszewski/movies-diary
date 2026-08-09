use crate::import::commands::DeleteImportProfileCommand;
use crate::import::deps::DeleteImportProfileDeps;
use domain::{
    errors::DomainError,
    value_objects::{ImportProfileId, UserId},
};

pub async fn execute(
    deps: &DeleteImportProfileDeps,
    cmd: DeleteImportProfileCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    deps.import_profile
        .get(&profile_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    deps.import_profile.delete(&profile_id).await
}

#[cfg(test)]
#[path = "tests/delete_profile.rs"]
mod tests;
