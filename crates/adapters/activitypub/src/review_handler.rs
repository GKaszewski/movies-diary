use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    models::ReviewSource,
    ports::LocalApContentQuery,
    value_objects::{Comment, MovieId, Rating, ReviewId, UserId},
};
use k_ap::{ApContentReader, ApObjectHandler};
use url::Url;

use crate::objects::{ReviewObject, review_to_ap_object};
use crate::remote_review_repository::RemoteReviewRepository;
use crate::urls::{actor_url, review_url};

pub struct ReviewObjectHandler {
    pub content_query: Arc<dyn LocalApContentQuery>,
    pub review_store: Arc<dyn RemoteReviewRepository>,
    pub base_url: String,
}

#[async_trait]
impl ApContentReader for ReviewObjectHandler {
    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        before: Option<chrono::DateTime<chrono::Utc>>,
        limit: usize,
    ) -> anyhow::Result<Vec<(url::Url, serde_json::Value, chrono::DateTime<chrono::Utc>)>> {
        let domain_user_id = UserId::from_uuid(user_id);
        let before_naive = before.map(|dt| dt.naive_utc());
        let entries = self
            .content_query
            .get_local_reviews_page(&domain_user_id, before_naive, limit)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let actor = actor_url(&self.base_url, user_id);
        let mut results = Vec::new();
        for entry in entries {
            let review = entry.review();
            let published =
                chrono::DateTime::from_naive_utc_and_offset(*review.watched_at(), chrono::Utc);
            let movie = entry.movie();
            let ap_id = review_url(&self.base_url, review.id());
            let poster_url = movie
                .poster_path()
                .map(|p| format!("{}/images/{}", self.base_url, p.value()));

            let obj = review_to_ap_object(
                review,
                ap_id.clone(),
                actor.clone(),
                movie.title().value().to_string(),
                movie.release_year().value(),
                poster_url,
                &self.base_url,
            );
            results.push((ap_id, serde_json::to_value(obj)?, published));
        }
        Ok(results)
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        self.content_query
            .count_local_posts()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}

#[async_trait]
impl ApObjectHandler for ReviewObjectHandler {
    async fn on_create(
        &self,
        _ap_id: &Url,
        _actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let obj: ReviewObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(e) => {
                tracing::debug!("ignoring unrecognized Create object: {}", e);
                return Ok(());
            }
        };

        let actor_url_str = obj.attributed_to.to_string();
        let review_id = ReviewId::generate();
        let movie_id = MovieId::from_uuid(uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            obj.movie_title.as_bytes(),
        ));
        let user_id = UserId::from_uuid(uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            actor_url_str.as_bytes(),
        ));
        let rating = Rating::new(obj.rating.min(5))?;
        let comment = obj.comment.map(Comment::new).transpose()?;

        let review = domain::models::Review::from_persistence(domain::models::PersistedReview {
            id: review_id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at: obj.watched_at.naive_utc(),
            created_at: obj.published.naive_utc(),
            source: ReviewSource::Remote {
                actor_url: actor_url_str,
            },
        });

        self.review_store
            .save_remote_review(
                &review,
                obj.id.as_str(),
                &obj.movie_title,
                obj.release_year,
                obj.poster_url.as_deref(),
            )
            .await?;

        Ok(())
    }

    async fn on_update(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let obj: ReviewObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(_) => {
                tracing::debug!(actor = %actor_url, "ignoring non-review Update activity");
                return Ok(());
            }
        };

        if obj.attributed_to != *actor_url {
            anyhow::bail!("update actor does not match object attributed_to");
        }

        self.review_store
            .update_remote_review(
                ap_id.as_str(),
                actor_url.as_str(),
                obj.rating.min(5),
                obj.comment.as_deref(),
                obj.watched_at.naive_utc(),
            )
            .await?;

        Ok(())
    }

    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()> {
        self.review_store
            .delete_remote_review(ap_id.as_str(), actor_url.as_str())
            .await
    }

    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()> {
        self.review_store.delete_by_actor(actor_url.as_str()).await
    }

    async fn on_like(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_announce_received(
        &self,
        _object_url: &Url,
        _actor_url: &Url,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_announce_of_remote(
        &self,
        _object_url: &Url,
        _actor_url: &Url,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_unlike(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_mention(
        &self,
        _thought_ap_id: &Url,
        _mentioned_user_uuid: uuid::Uuid,
        _actor_url: &Url,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
