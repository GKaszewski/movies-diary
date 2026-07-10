use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{
        DiaryEntry, FederationFlags, PendingFollowerInfo, RemoteActorInfo, RemoteGoalEntry,
        RemoteWatchlistEntry, WatchlistWithMovie,
    },
    value_objects::{MovieId, UserId},
};

#[async_trait]
pub trait SocialQueryPort: Send + Sync {
    async fn get_accepted_following_urls(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<String>, DomainError>;
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError>;
    async fn count_following(&self, user_id: &UserId) -> Result<usize, DomainError>;
    async fn count_accepted_followers(&self, user_id: &UserId) -> Result<usize, DomainError>;
    async fn get_pending_followers(
        &self,
        user_id: &UserId,
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

/// Federation-specific read-only queries that have no equivalent on the
/// standard domain ports (e.g. unpaginated watchlist, local-only review
/// listings). Generic lookups (get_movie_by_id, get_review_by_id, etc.)
/// live on MovieRepository, ReviewRepository, and the other domain ports.
#[async_trait]
pub trait LocalApContentQuery: Send + Sync {
    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError>;
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
}
