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
    fn name(&self) -> &'static str {
        "follow_backfill"
    }

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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use domain::models::FollowedActorInfo;
    use domain::value_objects::UserId;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct CallLog(Mutex<Vec<String>>);

    #[async_trait]
    impl ApBackfillPort for CallLog {
        async fn get_following(&self, _: Uuid) -> Result<Vec<FollowedActorInfo>, DomainError> {
            Ok(vec![FollowedActorInfo {
                url: "https://remote.example/users/a".into(),
                outbox_url: Some("https://remote.example/users/a/outbox".into()),
            }])
        }
        async fn import_remote_outbox(&self, outbox: &str, actor: &str) -> Result<(), DomainError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("import {outbox} {actor}"));
            Ok(())
        }
        async fn run_backfill_for_follower(
            &self,
            owner: Uuid,
            inbox: String,
        ) -> Result<(), DomainError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("backfill {owner} {inbox}"));
            Ok(())
        }
    }

    #[tokio::test]
    async fn follow_backfill_replays_identical_port_calls_when_run_twice() {
        let port = Arc::new(CallLog::default());
        let handler = FollowBackfillHandler {
            backfill: port.clone(),
        };
        let owner = UserId::from_uuid(Uuid::new_v4());
        let events = [
            DomainEvent::FollowAccepted {
                owner: owner.clone(),
                requester: SocialIdentity::Remote {
                    actor_url: "https://remote.example/users/a".into(),
                },
            },
            DomainEvent::BackfillFollower {
                owner_user_id: owner,
                follower_inbox_url: "https://remote.example/inbox".into(),
            },
        ];

        for event in &events {
            let start = port.0.lock().unwrap().len();
            handler.handle(event).await.unwrap();
            let mid = port.0.lock().unwrap().len();
            handler.handle(event).await.unwrap();
            let log = port.0.lock().unwrap().clone();
            assert!(mid > start, "the event reaches the port");
            assert_eq!(log[start..mid], log[mid..]);
        }
    }
}
