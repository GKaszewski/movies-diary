use activitypub::RemoteReviewRepository;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use domain::models::{Review, ReviewSource};

use super::{PostgresFederationRepository, datetime_to_str};

#[async_trait]
impl RemoteReviewRepository for PostgresFederationRepository {
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
        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES ($1, $2, $3, $4, NULL, $5)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = COALESCE(EXCLUDED.external_metadata_id, movies.external_metadata_id),
                 poster_path = COALESCE(EXCLUDED.poster_path, movies.poster_path)",
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
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, ap_id)
             VALUES ($1, $2, $3, $4, $5, $6::timestamptz, $7::timestamptz, $8, $9) ON CONFLICT DO NOTHING",
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
        sqlx::query("DELETE FROM reviews WHERE ap_id = $1 AND remote_actor_url = $2")
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
        watch_medium: Option<&str>,
    ) -> Result<()> {
        let watched_at_str = datetime_to_str(&watched_at);
        sqlx::query(
            "UPDATE reviews SET rating = $1, comment = $2, watched_at = $3::timestamptz, watch_medium = $4
             WHERE ap_id = $5 AND remote_actor_url = $6",
        )
        .bind(rating as i64)
        .bind(comment)
        .bind(&watched_at_str)
        .bind(watch_medium)
        .bind(ap_id)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;
        if let Some(url) = poster_url {
            sqlx::query(
                "UPDATE movies SET poster_path = $1
                 WHERE id = (SELECT movie_id FROM reviews WHERE ap_id = $2 AND remote_actor_url = $3)",
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
        sqlx::query("DELETE FROM reviews WHERE remote_actor_url = $1")
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
