use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Review, ReviewSource},
    ports::ReviewRepository,
    value_objects::{MovieId, ReviewId, UserId},
};
use sqlx::SqlitePool;

use rows::ReviewRow;

pub(crate) mod rows;
use crate::shared::{datetime_to_str, enqueue_events};

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
    async fn save_review(
        &self,
        review: &Review,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
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

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium, spoilers)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(review.spoilers())
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        let id = review_id.value().to_string();
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium, spoilers
             FROM reviews WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .map(ReviewRow::into_domain)
        .transpose()
    }

    async fn update_review(
        &self,
        review: &Review,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id = review.id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let watch_medium = review.watch_medium().map(|wm| wm.to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "UPDATE reviews SET rating = ?, comment = ?, watched_at = ?, watch_medium = ?, spoilers = ? WHERE id = ?",
        )
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&watch_medium)
        .bind(review.spoilers())
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn delete_review_and_orphaned_movie(
        &self,
        review_id: &ReviewId,
        movie_id: &MovieId,
        review_events: &[DomainEvent],
        orphan_events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id = review_id.value().to_string();
        let movie = movie_id.value().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query("DELETE FROM reviews WHERE id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, review_events).await?;
        let orphaned = sqlx::query(
            "DELETE FROM movies WHERE id = ? \
             AND NOT EXISTS (SELECT 1 FROM reviews WHERE movie_id = ?)",
        )
        .bind(&movie)
        .bind(&movie)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        if orphaned.rows_affected() > 0 {
            enqueue_events(&mut tx, orphan_events).await?;
        }
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, watch_medium, spoilers
             FROM reviews WHERE user_id = ? ORDER BY watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .into_iter()
        .map(ReviewRow::into_domain)
        .collect()
    }
}
