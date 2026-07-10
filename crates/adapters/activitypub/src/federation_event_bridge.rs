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
            } => self
                .domain_publisher
                .publish(&DomainEvent::BackfillFollower {
                    owner_user_id: UserId::from_uuid(owner_user_id),
                    follower_inbox_url,
                })
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            FederationEvent::DeliveryRequested {
                inbox,
                activity,
                signing_actor_id,
            } => self
                .domain_publisher
                .publish(&DomainEvent::FederationDeliveryRequested {
                    inbox_url: inbox.to_string(),
                    activity_json: activity,
                    signing_actor_id,
                })
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            FederationEvent::DeliveryFailed { inbox, error, .. } => {
                tracing::warn!(inbox = %inbox, error = %error, "federation delivery failed permanently");
                Ok(())
            }
            FederationEvent::OutboundFollowAccepted {
                local_user_id,
                remote_actor_url,
                outbox_url,
            } => {
                let identity = domain::value_objects::SocialIdentity::Remote {
                    actor_url: remote_actor_url,
                };
                self.domain_publisher
                    .publish(&DomainEvent::FollowAccepted {
                        owner: UserId::from_uuid(local_user_id),
                        requester: identity,
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;

                if let Some(outbox) = outbox_url {
                    tracing::info!(outbox = %outbox, "importing remote outbox after follow accepted");
                    // Handled by FollowBackfillHandler reacting to FollowAccepted
                }
                Ok(())
            }
        }
    }
}
