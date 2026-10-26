use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{events::DomainEvent, models::Review};

pub struct RemoteReviewUpdate<'a> {
    pub ap_id: &'a str,
    pub actor_url: &'a str,
    pub rating: u8,
    pub comment: Option<&'a str>,
    pub watched_at: NaiveDateTime,
    pub poster_url: Option<&'a str>,
    pub watch_medium: Option<&'a str>,
    pub spoilers: bool,
}

pub struct NewRemoteReview<'a> {
    pub review: &'a Review,
    pub ap_id: &'a str,
    pub movie_title: &'a str,
    pub release_year: u16,
    pub external_metadata_id: Option<&'a str>,
    pub poster_url: Option<&'a str>,
    pub events: &'a [DomainEvent],
}

#[async_trait]
pub trait RemoteReviewRepository: Send + Sync {
    async fn save_remote_review(&self, new: NewRemoteReview<'_>) -> Result<()>;

    async fn delete_remote_review(&self, ap_id: &str, actor_url: &str) -> Result<()>;

    async fn update_remote_review(&self, update: RemoteReviewUpdate<'_>) -> Result<()>;

    async fn delete_by_actor(&self, actor_url: &str) -> Result<()>;
}
