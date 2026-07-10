use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{Review, ReviewSource},
    ports::ReviewRepository,
    value_objects::{ReviewId, UserId},
};
use sqlx::SqlitePool;

use crate::models::ReviewRow;
use adapter_common::datetime_to_str;

pub struct SqliteReviewRepository {
    pool: SqlitePool,
}

impl SqliteReviewRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for SqliteReviewRepository {
    async fn save_review(&self, review: &Review) -> Result<(), DomainError> {
        let id = review.id().value().to_string();
        let movie_id = review.movie_id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());
        let remote_actor_url = match review.source() {
            ReviewSource::Local => None,
            ReviewSource::Remote { actor_url } => Some(actor_url.clone()),
        };

        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&movie_id)
        .bind(&user_id)
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&created_at)
        .bind(&remote_actor_url)
        .bind(review.watch_medium().map(|wm| wm.to_string()))
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        let id = review_id.value().to_string();
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium
             FROM reviews WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .map(ReviewRow::into_domain)
        .transpose()
    }

    async fn update_review(&self, review: &Review) -> Result<(), DomainError> {
        let id = review.id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let watch_medium = review.watch_medium().map(|wm| wm.to_string());

        sqlx::query(
            "UPDATE reviews SET rating = ?, comment = ?, watched_at = ?, watch_medium = ? WHERE id = ?",
        )
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&watch_medium)
        .bind(&id)
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        let id = review_id.value().to_string();
        sqlx::query("DELETE FROM reviews WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        Ok(())
    }

    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium
             FROM reviews WHERE user_id = ? ORDER BY watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(ReviewRow::into_domain)
        .collect()
    }
}
