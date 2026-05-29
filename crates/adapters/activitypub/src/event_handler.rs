use async_trait::async_trait;
use domain::ports::EventHandler;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::LocalApContentQuery,
    value_objects::{ReviewId, UserId},
};
use std::sync::Arc;

use k_ap::{ActivityPubService, ApVisibility};

use crate::objects::review_to_ap_object;
use crate::urls::{actor_url, review_url};

pub struct ActivityPubEventHandler {
    ap_service: Arc<ActivityPubService>,
    content_query: Arc<dyn LocalApContentQuery>,
    base_url: String,
}

impl ActivityPubEventHandler {
    pub fn new(
        ap_service: Arc<ActivityPubService>,
        content_query: Arc<dyn LocalApContentQuery>,
        base_url: String,
    ) -> Self {
        Self {
            ap_service,
            content_query,
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
            DomainEvent::ReviewUpdated {
                review_id, user_id, ..
            } => self
                .on_review_updated(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::ReviewDeleted { review_id, user_id } => self
                .on_review_deleted(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::UserUpdated { user_id } => self
                .ap_service
                .broadcast_actor_update(user_id.value())
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::WatchlistEntryAdded {
                user_id,
                movie_id,
                movie_title,
                release_year,
                external_metadata_id,
                added_at,
            } => self
                .on_watchlist_added(
                    user_id,
                    movie_id,
                    movie_title,
                    *release_year,
                    external_metadata_id,
                    added_at,
                )
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::WatchlistEntryRemoved { user_id, movie_id } => self
                .on_watchlist_removed(user_id, movie_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            _ => Ok(()),
        }
    }
}

impl ActivityPubEventHandler {
    async fn on_review_logged(&self, user_id: &UserId, review_id: &ReviewId) -> anyhow::Result<()> {
        let review = match self.content_query.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let ap_id = review_url(&self.base_url, review_id);
        let actor = actor_url(&self.base_url, user_id.value());

        let movie = self
            .content_query
            .get_movie_by_id(review.movie_id())
            .await
            .ok()
            .flatten();
        let movie_title = movie
            .as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie.as_ref().map(|m| m.release_year().value()).unwrap_or(0);
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
            &self.base_url,
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_create_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;

        Ok(())
    }

    async fn on_review_updated(
        &self,
        user_id: &UserId,
        review_id: &ReviewId,
    ) -> anyhow::Result<()> {
        let review = match self.content_query.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let ap_id = review_url(&self.base_url, review_id);
        let actor = actor_url(&self.base_url, user_id.value());

        let movie = self
            .content_query
            .get_movie_by_id(review.movie_id())
            .await
            .ok()
            .flatten();
        let movie_title = movie
            .as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie.as_ref().map(|m| m.release_year().value()).unwrap_or(0);
        let poster_url = movie
            .as_ref()
            .and_then(|m| m.poster_path())
            .map(|p| format!("{}/images/{}", self.base_url, p.value()));

        let obj = review_to_ap_object(
            &review,
            ap_id,
            actor,
            movie_title,
            release_year,
            poster_url,
            &self.base_url,
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_update_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;

        Ok(())
    }

    async fn on_review_deleted(
        &self,
        user_id: &UserId,
        review_id: &ReviewId,
    ) -> anyhow::Result<()> {
        let ap_id = review_url(&self.base_url, review_id);
        self.ap_service
            .broadcast_delete_to_followers(user_id.value(), ap_id)
            .await?;
        Ok(())
    }

    async fn on_watchlist_added(
        &self,
        user_id: &UserId,
        movie_id: &domain::value_objects::MovieId,
        movie_title: &str,
        release_year: u16,
        external_metadata_id: &Option<String>,
        added_at: &chrono::NaiveDateTime,
    ) -> anyhow::Result<()> {
        use crate::urls::watchlist_entry_url;
        let ap_id = watchlist_entry_url(&self.base_url, user_id.value(), movie_id.value());
        let actor = actor_url(&self.base_url, user_id.value());

        let poster_url = self
            .content_query
            .get_movie_by_id(movie_id)
            .await
            .ok()
            .flatten()
            .and_then(|m| {
                m.poster_path()
                    .map(|p| format!("{}/images/{}", self.base_url, p.value()))
            });

        let added_at_utc =
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(*added_at, chrono::Utc);
        let obj = crate::objects::watchlist_to_ap_object(
            ap_id.clone(),
            actor,
            movie_title.to_string(),
            release_year,
            external_metadata_id.clone(),
            poster_url,
            added_at_utc,
            &self.base_url,
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_create_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;
        Ok(())
    }

    async fn on_watchlist_removed(
        &self,
        user_id: &UserId,
        movie_id: &domain::value_objects::MovieId,
    ) -> anyhow::Result<()> {
        use crate::urls::watchlist_entry_url;
        let ap_id = watchlist_entry_url(&self.base_url, user_id.value(), movie_id.value());
        self.ap_service
            .broadcast_delete_to_followers(user_id.value(), ap_id)
            .await?;
        Ok(())
    }
}
