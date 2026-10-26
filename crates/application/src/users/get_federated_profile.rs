use domain::errors::DomainError;
use domain::models::FederatedProfile;
use uuid::Uuid;

use crate::users::deps::GetFederatedProfileDeps;

pub async fn execute(
    deps: &GetFederatedProfileDeps,
    user_id: Uuid,
) -> Result<Option<FederatedProfile>, DomainError> {
    match &deps.federated_profile {
        Some(fed_query) => fed_query.get_federated_profile(user_id).await,
        None => Ok(None),
    }
}
