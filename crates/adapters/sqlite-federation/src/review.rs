use activitypub::RemoteReviewRepository;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use domain::models::{Review, ReviewSource};

use super::{SqliteFederationRepository, datetime_to_str};

#[async_trait]
impl RemoteReviewRepository for SqliteFederationRepository {
    async fn save_remote_review(
        &self,
        review: &Review,
        ap_id: &str,
        movie_title: &str,
        release_year: u16,
        external_metadata_id: Option<&str>,
        poster_url: Option<&str>,
    ) -> Result<()> {
        let actor_url = match review.source() {
            ReviewSource::Remote { actor_url } => actor_url.clone(),
            ReviewSource::Local => {
                return Err(anyhow!("save_remote_review called with a local review"));
            }
        };
        let movie_id = review.movie_id().value().to_string();
        let _ = sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, NULL, ?)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = COALESCE(excluded.external_metadata_id, movies.external_metadata_id),
                 poster_path = COALESCE(excluded.poster_path, movies.poster_path)",
        )
        .bind(&movie_id)
        .bind(external_metadata_id)
        .bind(movie_title)
        .bind(release_year.max(1888) as i64)
        .bind(poster_url)
        .execute(&self.pool)
        .await?;

        let id = review.id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());
        sqlx::query(
            "INSERT OR IGNORE INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, ap_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&movie_id)
        .bind(&user_id)
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&created_at)
        .bind(&actor_url)
        .bind(ap_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_remote_review(&self, ap_id: &str, actor_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM reviews WHERE ap_id = ? AND remote_actor_url = ?")
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_remote_review(
        &self,
        ap_id: &str,
        actor_url: &str,
        rating: u8,
        comment: Option<&str>,
        watched_at: chrono::NaiveDateTime,
        poster_url: Option<&str>,
    ) -> Result<()> {
        let watched_at_str = datetime_to_str(&watched_at);
        sqlx::query(
            "UPDATE reviews SET rating = ?, comment = ?, watched_at = ?
             WHERE ap_id = ? AND remote_actor_url = ?",
        )
        .bind(rating as i64)
        .bind(comment)
        .bind(&watched_at_str)
        .bind(ap_id)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;
        if let Some(url) = poster_url {
            sqlx::query(
                "UPDATE movies SET poster_path = ?
                 WHERE id = (SELECT movie_id FROM reviews WHERE ap_id = ? AND remote_actor_url = ?)",
            )
            .bind(url)
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn delete_by_actor(&self, actor_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM reviews WHERE remote_actor_url = ?")
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
