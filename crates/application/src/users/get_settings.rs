use std::sync::Arc;

use domain::{errors::DomainError, models::UserSettings, ports::UserSettingsRepository, value_objects::UserId};

pub async fn execute(
    user_settings: Arc<dyn UserSettingsRepository>,
    user_id: uuid::Uuid,
) -> Result<UserSettings, DomainError> {
    let uid = UserId::from_uuid(user_id);
    user_settings.get(&uid).await
}

#[cfg(test)]
#[path = "tests/get_settings.rs"]
mod tests;
