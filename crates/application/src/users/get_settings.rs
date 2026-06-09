use domain::{errors::DomainError, models::UserSettings, value_objects::UserId};

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, user_id: uuid::Uuid) -> Result<UserSettings, DomainError> {
    let uid = UserId::from_uuid(user_id);
    ctx.repos.user_settings.get(&uid).await
}

#[cfg(test)]
#[path = "tests/get_settings.rs"]
mod tests;
