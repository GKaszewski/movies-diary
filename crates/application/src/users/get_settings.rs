use domain::{errors::DomainError, models::UserSettings, value_objects::UserId};

use crate::users::deps::GetSettingsDeps;

pub async fn execute(
    deps: &GetSettingsDeps,
    user_id: uuid::Uuid,
) -> Result<UserSettings, DomainError> {
    let uid = UserId::from_uuid(user_id);
    deps.user_settings.get(&uid).await
}

#[cfg(test)]
#[path = "tests/get_settings.rs"]
mod tests;
