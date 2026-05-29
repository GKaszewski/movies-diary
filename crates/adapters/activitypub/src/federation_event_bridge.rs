use std::sync::Arc;

use domain::events::DomainEvent;
use domain::value_objects::UserId;
use k_ap::FederationEvent;

pub struct FederationEventBridge {
    domain_publisher: Arc<dyn domain::ports::EventPublisher>,
}

impl FederationEventBridge {
    pub fn new(domain_publisher: Arc<dyn domain::ports::EventPublisher>) -> Self {
        Self { domain_publisher }
    }
}

#[async_trait::async_trait]
impl k_ap::EventPublisher for FederationEventBridge {
    async fn publish(&self, event: FederationEvent) -> anyhow::Result<()> {
        match event {
            FederationEvent::BackfillRequested {
                owner_user_id,
                follower_inbox_url,
            } => {
                self.domain_publisher
                    .publish(&DomainEvent::BackfillFollower {
                        owner_user_id: UserId::from_uuid(owner_user_id),
                        follower_inbox_url,
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
            }
            _ => {
                tracing::debug!("ignoring federation event: {:?}", event);
                Ok(())
            }
        }
    }
}
