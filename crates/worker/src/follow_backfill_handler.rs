use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{ApBackfillPort, EventHandler},
    value_objects::SocialIdentity,
};

pub struct FollowBackfillHandler {
    pub backfill: Arc<dyn ApBackfillPort>,
}

#[async_trait]
impl EventHandler for FollowBackfillHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::FollowAccepted {
                owner,
                requester: SocialIdentity::Remote { actor_url },
            } => {
                tracing::info!(actor = %actor_url, "follow accepted — looking up outbox for import");
                let following = self.backfill.get_following(owner.value()).await?;
                if let Some(actor) = following.iter().find(|a| a.url == *actor_url) {
                    if let Some(outbox_url) = &actor.outbox_url {
                        tracing::info!(outbox = %outbox_url, actor = %actor_url, "importing remote outbox");
                        self.backfill
                            .import_remote_outbox(outbox_url, actor_url)
                            .await?;
                    } else {
                        tracing::warn!(actor = %actor_url, "no outbox URL for accepted follow — skipping import");
                    }
                }
                Ok(())
            }
            DomainEvent::BackfillFollower {
                owner_user_id,
                follower_inbox_url,
            } => {
                tracing::info!(owner = %owner_user_id.value(), inbox = %follower_inbox_url, "backfilling local content to new follower");
                self.backfill
                    .run_backfill_for_follower(owner_user_id.value(), follower_inbox_url.clone())
                    .await
            }
            _ => Ok(()),
        }
    }
}
