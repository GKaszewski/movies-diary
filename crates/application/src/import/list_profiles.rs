use crate::context::AppContext;
use domain::{errors::DomainError, models::ImportProfile, value_objects::UserId};

pub async fn execute(
    ctx: &AppContext,
    user_id: &UserId,
) -> Result<Vec<ImportProfile>, DomainError> {
    ctx.repos.import_profile.list_for_user(user_id).await
}

#[cfg(test)]
#[path = "tests/list_profiles.rs"]
mod tests;
