use activitypub_federation::{
    activity_sending::SendActivityTask,
    fetch::object_id::ObjectId,
    protocol::context::WithContext,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ReviewId, UserId},
};
use event_publisher::EventHandler;
use url::Url;

use crate::{
    activities::CreateActivity,
    actors::{actor_url, get_local_actor},
    federation::ApFederationConfig,
    objects::{review_url, ReviewObject},
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
                rating,
                watched_at,
                ..
            } => self
                .on_review_logged(user_id, review_id, rating.value(), *watched_at)
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
        rating: u8,
        watched_at: chrono::NaiveDateTime,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        let followers = data.federation_repo.get_followers(user_id.clone()).await?;
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let local_actor = get_local_actor(user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let review_id_url = review_url(&self.base_url, review_id);
        let actor_id = actor_url(&self.base_url, user_id);
        let activity_id = Url::parse(&format!(
            "{}/activities/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;

        let stars = "\u{2B50}".repeat(rating as usize);
        let now = DateTime::from_naive_utc_and_offset(watched_at, Utc);

        let object = ReviewObject {
            kind: "Review".to_string(),
            id: review_id_url.into(),
            attributed_to: actor_id.into(),
            content: format!("{} (movie review)", stars),
            published: Utc::now(),
            movie_title: "Unknown".to_string(), // TODO: fetch from MovieRepository
            rating,
            comment: None,
            watched_at: now,
        };

        let create = CreateActivity {
            id: activity_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };
        let create_with_ctx = WithContext::new_default(create);

        let inboxes: Vec<Url> = accepted
            .iter()
            .filter_map(|f| Url::parse(&f.actor.inbox_url).ok())
            .collect();

        let sends =
            SendActivityTask::prepare(&create_with_ctx, &local_actor, inboxes, &data).await?;
        for send in sends {
            if let Err(e) = send.sign_and_send(&data).await {
                tracing::warn!(error = %e, "failed to deliver activity to follower");
            }
        }

        Ok(())
    }
}
