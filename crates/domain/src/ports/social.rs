use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{
        DiaryEntry, FederationFlags, Goal, Movie, PendingFollowerInfo, RemoteActorInfo,
        RemoteGoalEntry, RemoteWatchlistEntry, Review, WatchlistWithMovie,
    },
    value_objects::{MovieId, ReviewId, UserId},
};

#[async_trait]
pub trait SocialQueryPort: Send + Sync {
    async fn get_accepted_following_urls(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<String>, DomainError>;
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError>;
    async fn count_following(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;
    async fn count_accepted_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;
    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<PendingFollowerInfo>, DomainError>;
}

#[async_trait]
pub trait UserFederationSettingsQuery: Send + Sync {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError>;
}

#[async_trait]
pub trait RemoteWatchlistRepository: Send + Sync {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), DomainError>;
    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError>;
    /// Find entries for a remote actor whose URL hashes (v5 UUID) to the given UUID.
    async fn get_by_derived_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
}

#[async_trait]
pub trait RemoteGoalRepository: Send + Sync {
    async fn save(&self, entry: RemoteGoalEntry) -> Result<(), DomainError>;
    async fn update_by_ap_id(
        &self,
        ap_id: &str,
        target: u32,
        current: u32,
    ) -> Result<(), DomainError>;
    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError>;
    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_actor_url(&self, actor_url: &str) -> Result<Vec<RemoteGoalEntry>, DomainError>;
}

/// Read-only query port used exclusively by the ActivityPub adapter.
/// Consolidates all reads the AP adapter needs so it never touches write repositories.
#[async_trait]
pub trait LocalApContentQuery: Send + Sync {
    async fn get_local_reviews_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError>;
    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError>;
    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError>;
    async fn get_movie_by_external_metadata_id(
        &self,
        external_id: &str,
    ) -> Result<Option<Movie>, DomainError>;
    async fn count_local_posts(&self) -> Result<u64, DomainError>;
    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
    async fn get_local_reviews_page(
        &self,
        user_id: &UserId,
        before: Option<NaiveDateTime>,
        limit: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
    async fn get_goal_with_progress(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<(Goal, u32)>, DomainError>;
    async fn list_goals_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError>;
}
