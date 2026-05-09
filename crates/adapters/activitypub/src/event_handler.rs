use activitypub_federation::{
    activity_sending::SendActivityTask,
    fetch::object_id::ObjectId,
    protocol::context::WithContext,
    traits::Object,
};
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ReviewId, UserId},
};
use event_publisher::EventHandler;
use url::Url;

use crate::{
    activities::CreateActivity,
    actors::get_local_actor,
    federation::ApFederationConfig,
    objects::DbReview,
    repository::FollowerStatus,
};

pub struct ActivityPubEventHandler {
    federation_config: ApFederationConfig,
    base_url: String,
}

impl ActivityPubEventHandler {
    pub fn new(federation_config: ApFederationConfig, base_url: String) -> Self {
        Self {
            federation_config,
            base_url,
        }
    }
}

#[async_trait]
impl EventHandler for ActivityPubEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::ReviewLogged {
                review_id,
                user_id,
                ..
            } => self
                .on_review_logged(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            _ => Ok(()),
        }
    }
}

impl ActivityPubEventHandler {
    async fn on_review_logged(
        &self,
        user_id: &UserId,
        review_id: &ReviewId,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        let followers = data.federation_repo.get_followers(user_id.clone()).await?;
        tracing::debug!(user_id = %user_id.value(), count = followers.len(), "AP: got followers for review");

        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .collect();

        tracing::debug!(accepted = accepted.len(), "AP: accepted followers");

        if accepted.is_empty() {
            return Ok(());
        }

        let review = match data.movie_repo.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let local_actor = get_local_actor(user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let activity_id = crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;

        let db_review = DbReview {
            ap_id: crate::urls::review_url(&self.base_url, review_id),
            review,
        };
        let object = db_review
            .into_json(&data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let create = CreateActivity {
            id: activity_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };
        let create_with_ctx = WithContext::new_default(create);

        let inboxes: Vec<Url> = accepted
            .iter()
            .filter_map(|f| {
                let url = Url::parse(&f.actor.inbox_url);
                if url.is_err() {
                    tracing::warn!(inbox = %f.actor.inbox_url, "AP: invalid inbox URL, skipping follower");
                }
                url.ok()
            })
            .collect();

        tracing::debug!(inboxes = inboxes.len(), "AP: delivering to inboxes");

        let sends =
            SendActivityTask::prepare(&create_with_ctx, &local_actor, inboxes, &data).await?;
        tracing::debug!(sends = sends.len(), "AP: prepared sends");
        let failures = crate::service::send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some activity deliveries failed permanently");
        }

        Ok(())
    }
}
