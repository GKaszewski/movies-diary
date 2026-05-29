use std::sync::Arc;

use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventHandler};

pub struct FollowBackfillHandler {
    pub ap_service: Arc<dyn activitypub::ActivityPubPort>,
}

#[async_trait]
impl EventHandler for FollowBackfillHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::FollowAccepted {
                remote_actor_url,
                outbox_url,
                ..
            } => {
                tracing::info!(actor = %remote_actor_url, outbox = %outbox_url, "importing remote outbox");
                self.ap_service
                    .import_remote_outbox(outbox_url, remote_actor_url)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))
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
