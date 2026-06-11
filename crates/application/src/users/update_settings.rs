use std::sync::Arc;

use domain::{errors::DomainError, ports::UserSettingsRepository, value_objects::UserId};

pub struct UpdateUserSettingsCommand {
    pub user_id: uuid::Uuid,
    pub federate_goals: bool,
}

pub async fn execute(
    user_settings: Arc<dyn UserSettingsRepository>,
    cmd: UpdateUserSettingsCommand,
) -> Result<(), DomainError> {
    let uid = UserId::from_uuid(cmd.user_id);
    let mut settings = user_settings.get(&uid).await?;
    settings.set_federate_goals(cmd.federate_goals);
    user_settings.save(&settings).await
}

#[cfg(test)]
#[path = "tests/update_settings.rs"]
mod tests;
