use async_trait::async_trait;

use crate::{errors::DomainError, value_objects::UserId};

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

// ── NoopSocialQueryPort ───────────────────────────────────────────────────────

/// Stub used when federation is disabled — returns empty results.
pub struct NoopSocialQueryPort;

#[async_trait]
impl super::SocialQueryPort for NoopSocialQueryPort {
    async fn get_accepted_following_urls(&self, _: &UserId) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<crate::models::RemoteActorInfo>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_accepted_followers(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_pending_followers(
        &self,
        _: &UserId,
    ) -> Result<Vec<crate::models::PendingFollowerInfo>, DomainError> {
        Ok(vec![])
    }
}
