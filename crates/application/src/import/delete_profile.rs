use crate::{context::AppContext, import::commands::DeleteImportProfileCommand};
use domain::{
    errors::DomainError,
    value_objects::{ImportProfileId, UserId},
};

pub async fn execute(ctx: &AppContext, cmd: DeleteImportProfileCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let profile_id = ImportProfileId::from_uuid(cmd.profile_id);

    ctx.repos
        .import_profile
        .get(&profile_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import profile".into()))?;
    ctx.repos.import_profile.delete(&profile_id).await
}
