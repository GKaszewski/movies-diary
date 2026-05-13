use std::sync::Arc;

use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventHandler};

pub struct FollowBackfillHandler {
    pub ap_service: Arc<dyn activitypub::ActivityPubPort>,
}

#[async_trait]
impl EventHandler for FollowBackfillHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let DomainEvent::FollowAccepted {
            remote_actor_url,
            outbox_url,
            ..
        } = event
        else {
            return Ok(());
        };
        tracing::info!(actor = %remote_actor_url, outbox = %outbox_url, "starting outbox backfill");
        self.ap_service
            .backfill_outbox(outbox_url, remote_actor_url)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}
