use async_trait::async_trait;

use crate::{errors::DomainError, models::FederatedProfile};

#[async_trait]
pub trait FederatedProfileQuery: Send + Sync {
    async fn get_federated_profile(
        &self,
        synthetic_user_id: uuid::Uuid,
    ) -> Result<Option<FederatedProfile>, DomainError>;
}
