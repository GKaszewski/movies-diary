use domain::{errors::DomainError, value_objects::{ImportProfileId, UserId}};
use crate::{commands::DeleteImportProfileCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: DeleteImportProfileCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    ctx.import_profile_repository
        .get(&profile_id, &user_id).await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    ctx.import_profile_repository.delete(&profile_id).await
}
