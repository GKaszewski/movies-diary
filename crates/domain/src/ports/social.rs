use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{
        DiaryEntry, FederationFlags, PendingFollowerInfo, RemoteActorInfo, RemoteGoalEntry,
        RemoteWatchlistEntry, WatchlistWithMovie,
    },
    value_objects::{MovieId, SocialIdentity, UserId},
};

// ── Unified social ports (ADR-0002) ─────────────────────────────────────────

#[async_trait]
pub trait SocialCommand: Send + Sync {
    async fn follow(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn unfollow(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn block(
        &self,
        blocker: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn unblock(
        &self,
        blocker: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait SocialQuery: Send + Sync {
    async fn get_following(
        &self,
        user: &UserId,
    ) -> Result<Vec<SocialIdentity>, DomainError>;

    async fn get_followers(
        &self,
        user: &UserId,
    ) -> Result<Vec<SocialIdentity>, DomainError>;

    async fn get_pending_followers(
        &self,
        user: &UserId,
    ) -> Result<Vec<SocialIdentity>, DomainError>;

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError>;

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError>;

    async fn get_blocked(
        &self,
        user: &UserId,
    ) -> Result<Vec<SocialIdentity>, DomainError>;

    async fn is_following(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<bool, DomainError>;
}

// ── Legacy ports (pre-unification, still used by AP adapter + handlers) ─────

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
