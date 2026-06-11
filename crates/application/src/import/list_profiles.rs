use std::sync::Arc;

use domain::{
    errors::DomainError, models::ImportProfile, ports::ImportProfileRepository,
    value_objects::UserId,
};

pub async fn execute(
    import_profile: Arc<dyn ImportProfileRepository>,
    user_id: &UserId,
) -> Result<Vec<ImportProfile>, DomainError> {
    import_profile.list_for_user(user_id).await
}

#[cfg(test)]
#[path = "tests/list_profiles.rs"]
mod tests;
