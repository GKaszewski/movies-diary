use domain::{errors::DomainError, value_objects::UserId};

use crate::users::deps::UpdateSettingsDeps;

pub struct UpdateUserSettingsCommand {
    pub user_id: uuid::Uuid,
    pub federate_goals: bool,
    pub federate_reviews: bool,
    pub federate_watchlist: bool,
}

pub async fn execute(
    deps: &UpdateSettingsDeps,
    cmd: UpdateUserSettingsCommand,
) -> Result<(), DomainError> {
    let uid = UserId::from_uuid(cmd.user_id);
    let mut settings = deps.user_settings.get(&uid).await?;
    settings.set_federate_goals(cmd.federate_goals);
    settings.set_federate_reviews(cmd.federate_reviews);
    settings.set_federate_watchlist(cmd.federate_watchlist);
    deps.user_settings.save(&settings).await
}

#[cfg(test)]
#[path = "tests/update_settings.rs"]
mod tests;
