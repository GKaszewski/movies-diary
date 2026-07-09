use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::models::Review;

pub struct RemoteReviewUpdate<'a> {
    pub ap_id: &'a str,
    pub actor_url: &'a str,
    pub rating: u8,
    pub comment: Option<&'a str>,
    pub watched_at: NaiveDateTime,
    pub poster_url: Option<&'a str>,
    pub watch_medium: Option<&'a str>,
}

#[async_trait]
pub trait RemoteReviewRepository: Send + Sync {
    async fn save_remote_review(
        &self,
        review: &Review,
        ap_id: &str,
        movie_title: &str,
        release_year: u16,
        external_metadata_id: Option<&str>,
        poster_url: Option<&str>,
    ) -> Result<()>;

    async fn delete_remote_review(&self, ap_id: &str, actor_url: &str) -> Result<()>;

    async fn update_remote_review(&self, update: RemoteReviewUpdate<'_>) -> Result<()>;

    async fn delete_by_actor(&self, actor_url: &str) -> Result<()>;
}
