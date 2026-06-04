use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::models::Review;

#[async_trait]
pub trait RemoteReviewRepository: Send + Sync {
    async fn save_remote_review(
        &self,
        review: &Review,
        ap_id: &str,
        movie_title: &str,
        release_year: u16,
        poster_url: Option<&str>,
    ) -> Result<()>;

    async fn delete_remote_review(&self, ap_id: &str, actor_url: &str) -> Result<()>;

    async fn update_remote_review(
        &self,
        ap_id: &str,
        actor_url: &str,
        rating: u8,
        comment: Option<&str>,
        watched_at: NaiveDateTime,
        poster_url: Option<&str>,
    ) -> Result<()>;

    async fn delete_by_actor(&self, actor_url: &str) -> Result<()>;
}
