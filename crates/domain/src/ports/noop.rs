use async_trait::async_trait;

use crate::{
    errors::DomainError,
    value_objects::{SocialActor, SocialIdentity, UserId},
};

// ── NoopRemoteWatchlistRepository ─────────────────────────────────────────────

/// Stub used when federation is disabled — every operation is a no-op.
pub struct NoopRemoteWatchlistRepository;

#[async_trait]
impl super::RemoteWatchlistRepository for NoopRemoteWatchlistRepository {
    async fn save(&self, _: crate::models::RemoteWatchlistEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_derived_uuid(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
}

// ── NoopSocialCommand ────────────────────────────────────────────────────────

pub struct NoopSocialCommand;

#[async_trait]
impl super::SocialCommand for NoopSocialCommand {
    async fn follow(&self, _: &UserId, _: &crate::value_objects::FollowTarget) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unfollow(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn accept_follow(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reject_follow(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_follower(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn block(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unblock(&self, _: &UserId, _: &SocialIdentity) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── NoopSocialQuery ─────────────────────────────────────────────────────────

pub struct NoopSocialQuery;

#[async_trait]
impl super::SocialQuery for NoopSocialQuery {
    async fn get_following(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_followers(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_followers(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_followers(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_blocked(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn is_following(&self, _: &UserId, _: &SocialIdentity) -> Result<bool, DomainError> {
        Ok(false)
    }
}

// ── NoopFederationAdminQuery ─────────────────────────────────────────────────

/// Stub used when federation is disabled — returns empty results.
pub struct NoopFederationAdminQuery;

#[async_trait]
impl super::FederationAdminQuery for NoopFederationAdminQuery {
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<crate::models::RemoteActorInfo>, DomainError> {
        Ok(vec![])
    }
}
