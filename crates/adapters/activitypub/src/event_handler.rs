use async_trait::async_trait;
use domain::ports::EventHandler;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{MovieRepository, ReviewRepository},
    value_objects::{ReviewId, UserId},
};
use std::sync::Arc;

use activitypub_base::ActivityPubService;

use crate::objects::review_to_ap_object;
use crate::urls::{actor_url, review_url};

pub struct ActivityPubEventHandler {
    ap_service: Arc<ActivityPubService>,
    movie_repository: Arc<dyn MovieRepository>,
    review_repository: Arc<dyn ReviewRepository>,
    base_url: String,
}

impl ActivityPubEventHandler {
    pub fn new(
        ap_service: Arc<ActivityPubService>,
        movie_repository: Arc<dyn MovieRepository>,
        review_repository: Arc<dyn ReviewRepository>,
        base_url: String,
    ) -> Self {
        Self {
            ap_service,
            movie_repository,
            review_repository,
            base_url,
        }
    }
}

#[async_trait]
impl EventHandler for ActivityPubEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::ReviewLogged {
                review_id, user_id, ..
            } => self
                .on_review_logged(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::UserUpdated { user_id } => self
                .ap_service
                .broadcast_actor_update(user_id.value())
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            _ => Ok(()),
        }
    }
}

impl ActivityPubEventHandler {
    async fn on_review_logged(&self, user_id: &UserId, review_id: &ReviewId) -> anyhow::Result<()> {
        let review = match self.review_repository.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let ap_id = review_url(&self.base_url, review_id);
        let actor = actor_url(&self.base_url, user_id.value());

        let movie = self
            .movie_repository
            .get_movie_by_id(review.movie_id())
            .await
            .ok()
            .flatten();
        let movie_title = movie
            .as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie
            .as_ref()
            .map(|m| m.release_year().value())
            .unwrap_or(0);
        let poster_url = movie
            .as_ref()
            .and_then(|m| m.poster_path())
            .map(|p| format!("{}/images/{}", self.base_url, p.value()));

        let obj = review_to_ap_object(
            &review,
            ap_id.clone(),
            actor,
            movie_title,
            release_year,
            poster_url,
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_to_followers(user_id.value(), ap_id, json)
            .await?;

        Ok(())
    }
}
