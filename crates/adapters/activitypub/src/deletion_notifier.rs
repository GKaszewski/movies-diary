use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::FederationDeletionNotifier,
    value_objects::{InstanceIdentity, UserId},
};
use k_ap::ActivityPubService;

use crate::urls::actor_url;

pub struct ApDeletionNotifier {
    service: Arc<ActivityPubService>,
    instance: InstanceIdentity,
}

impl ApDeletionNotifier {
    pub fn new(service: Arc<ActivityPubService>, instance: InstanceIdentity) -> Self {
        Self { service, instance }
    }
}

#[async_trait]
impl FederationDeletionNotifier for ApDeletionNotifier {
    async fn notify_deleted(&self, user: &UserId) -> Result<(), DomainError> {
        let ap_id = actor_url(&self.instance, user.value());
        self.service
            .broadcast_delete_to_followers(user.value(), ap_id)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}
