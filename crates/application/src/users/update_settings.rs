use domain::{errors::DomainError, value_objects::UserId};

use crate::context::AppContext;

pub struct UpdateUserSettingsCommand {
    pub user_id: uuid::Uuid,
    pub federate_goals: bool,
}

pub async fn execute(ctx: &AppContext, cmd: UpdateUserSettingsCommand) -> Result<(), DomainError> {
    let uid = UserId::from_uuid(cmd.user_id);
    let mut settings = ctx.repos.user_settings.get(&uid).await?;
    settings.set_federate_goals(cmd.federate_goals);
    ctx.repos.user_settings.save(&settings).await
}

#[cfg(test)]
#[path = "tests/update_settings.rs"]
mod tests;
