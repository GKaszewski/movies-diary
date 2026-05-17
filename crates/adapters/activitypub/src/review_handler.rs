use std::sync::Arc;

use k_ap::ApObjectHandler;
use async_trait::async_trait;
use domain::{
    models::{Review, ReviewSource},
    ports::{DiaryRepository, MovieRepository},
    value_objects::{Comment, MovieId, Rating, ReviewId, UserId},
};
use url::Url;

use crate::objects::{ReviewObject, review_to_ap_object};
use crate::remote_review_repository::RemoteReviewRepository;
use crate::urls::{actor_url, review_url};

pub struct ReviewObjectHandler {
    pub movie_repository: Arc<dyn MovieRepository>,
    pub diary_repository: Arc<dyn DiaryRepository>,
    pub review_store: Arc<dyn RemoteReviewRepository>,
    pub base_url: String,
}

#[async_trait]
impl ApObjectHandler for ReviewObjectHandler {
    async fn get_local_objects_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value)>> {
        let domain_user_id = UserId::from_uuid(user_id);
        let history = self
            .diary_repository
            .get_user_history(&domain_user_id)
            .await?;

        let mut results = Vec::new();
        for entry in history {
            let review = entry.review();
            if !matches!(review.source(), ReviewSource::Local) {
                continue;
            }

            let ap_id = review_url(&self.base_url, review.id());
            let actor_url = actor_url(&self.base_url, user_id);

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
                review,
                ap_id.clone(),
                actor_url,
                movie_title,
                release_year,
                poster_url,
                &self.base_url,
            );
            let json = serde_json::to_value(obj)?;
            results.push((ap_id, json));
        }
        Ok(results)
    }

    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        before: Option<chrono::DateTime<chrono::Utc>>,
        limit: usize,
    ) -> anyhow::Result<Vec<(url::Url, serde_json::Value, chrono::DateTime<chrono::Utc>)>> {
        use domain::value_objects::UserId;

        let domain_user_id = UserId::from_uuid(user_id);
        let history = self
            .diary_repository
            .get_user_history(&domain_user_id)
            .await?;

        let mut results = Vec::new();
        for entry in history {
            let review = entry.review();
            if !matches!(review.source(), ReviewSource::Local) {
                continue;
            }

            let published =
                chrono::DateTime::from_naive_utc_and_offset(*review.watched_at(), chrono::Utc);

            if let Some(cutoff) = before
                && published >= cutoff
            {
                continue;
            }

            let ap_id = review_url(&self.base_url, review.id());
            let actor_url = actor_url(&self.base_url, user_id);

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
                review,
                ap_id.clone(),
                actor_url,
                movie_title,
                release_year,
                poster_url,
                &self.base_url,
            );
            let json = serde_json::to_value(obj)?;
            results.push((ap_id, json, published));

            if results.len() >= limit {
                break;
            }
        }
        Ok(results)
    }

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

        let review = Review::from_persistence(
            review_id,
            movie_id,
            user_id,
            rating,
            comment,
            obj.watched_at.naive_utc(),
            obj.published.naive_utc(),
            ReviewSource::Remote {
                actor_url: actor_url_str,
            },
        );

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

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        self.diary_repository
            .count_local_posts()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    async fn on_like(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_announce_received(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_unlike(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_mention(&self, _thought_ap_id: &Url, _mentioned_user_uuid: uuid::Uuid, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }
}
