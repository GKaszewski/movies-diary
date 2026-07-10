use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError, events::DomainEvent, ports::EventHandler, value_objects::SocialIdentity,
};

pub struct FollowBackfillHandler {
    pub ap_service: Arc<dyn activitypub::ActivityPubPort>,
}

#[async_trait]
impl EventHandler for FollowBackfillHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::FollowAccepted {
                requester: SocialIdentity::Remote { actor_url },
                ..
            } => {
                tracing::info!(actor = %actor_url, "follow accepted from remote actor");
                Ok(())
            }
            DomainEvent::BackfillFollower {
                owner_user_id,
                follower_inbox_url,
            } => {
                tracing::info!(owner = %owner_user_id.value(), inbox = %follower_inbox_url, "backfilling local content to new follower");
                self.ap_service
                    .run_backfill_for_follower(owner_user_id.value(), follower_inbox_url.clone())
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))
            }
            _ => Ok(()),
        }
    }
}
