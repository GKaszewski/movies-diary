use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::models::Review;
use domain::value_objects::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowerStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowingStatus {
    Pending,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteActor {
    pub url: String,
    pub handle: String,
    pub inbox_url: String,
    pub shared_inbox_url: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Follower {
    pub actor: RemoteActor,
    pub status: FollowerStatus,
}

#[async_trait]
pub trait FederationRepository: Send + Sync {
    async fn add_follower(&self, local_user_id: UserId, remote_actor_url: &str, status: FollowerStatus, follow_activity_id: &str) -> Result<()>;
    async fn get_follower_follow_activity_id(&self, local_user_id: UserId, remote_actor_url: &str) -> Result<Option<String>>;
    async fn remove_follower(&self, local_user_id: UserId, remote_actor_url: &str) -> Result<()>;
    async fn get_followers(&self, local_user_id: UserId) -> Result<Vec<Follower>>;
    async fn update_follower_status(&self, local_user_id: UserId, remote_actor_url: &str, status: FollowerStatus) -> Result<()>;
    async fn add_following(&self, local_user_id: UserId, actor: RemoteActor, follow_activity_id: &str) -> Result<()>;
    async fn get_follow_activity_id(&self, local_user_id: UserId, remote_actor_url: &str) -> Result<Option<String>>;
    async fn remove_following(&self, local_user_id: UserId, actor_url: &str) -> Result<()>;
    async fn get_following(&self, local_user_id: UserId) -> Result<Vec<RemoteActor>>;
    async fn count_following(&self, local_user_id: UserId) -> Result<usize>;
    async fn upsert_remote_actor(&self, actor: RemoteActor) -> Result<()>;
    async fn get_remote_actor(&self, actor_url: &str) -> Result<Option<RemoteActor>>;
    async fn save_remote_review(
        &self,
        review: &Review,
        ap_id: &str,
        movie_title: &str,
        release_year: u16,
        poster_url: Option<&str>,
    ) -> Result<()>;
    async fn delete_remote_review_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<()>;
    async fn update_remote_review(
        &self,
        ap_id: &str,
        actor_url: &str,
        rating: u8,
        comment: Option<&str>,
        watched_at: NaiveDateTime,
    ) -> Result<()>;
    async fn get_local_actor_keypair(&self, user_id: UserId) -> Result<Option<(String, String)>>;
    async fn save_local_actor_keypair(&self, user_id: UserId, public_key: String, private_key: String) -> Result<()>;
    async fn delete_remote_reviews_by_actor(&self, actor_url: &str) -> Result<()>;
    async fn get_pending_followers(&self, local_user_id: UserId) -> Result<Vec<RemoteActor>>;
    async fn update_following_status(
        &self,
        local_user_id: UserId,
        remote_actor_url: &str,
        status: FollowingStatus,
    ) -> Result<()>;
}
