use std::sync::Arc;

use crate::import::commands::DeleteImportProfileCommand;
use domain::{
    errors::DomainError,
    ports::ImportProfileRepository,
    value_objects::{ImportProfileId, UserId},
};

pub async fn execute(
    import_profile: Arc<dyn ImportProfileRepository>,
    cmd: DeleteImportProfileCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    import_profile
        .get(&profile_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    import_profile.delete(&profile_id).await
}

#[cfg(test)]
#[path = "tests/delete_profile.rs"]
mod tests;
