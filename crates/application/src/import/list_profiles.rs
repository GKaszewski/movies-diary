use crate::import::deps::ListImportProfilesDeps;
use domain::{errors::DomainError, models::ImportProfile, value_objects::UserId};

pub async fn execute(
    deps: &ListImportProfilesDeps,
    user_id: &UserId,
) -> Result<Vec<ImportProfile>, DomainError> {
    deps.import_profile.list_for_user(user_id).await
}

#[cfg(test)]
#[path = "tests/list_profiles.rs"]
mod tests;
