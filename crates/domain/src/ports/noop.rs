use async_trait::async_trait;

use crate::{
    errors::DomainError,
    value_objects::{FollowRelation, SocialActor, SocialIdentity, UserId},
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
    async fn follow(
        &self,
        _: &UserId,
        _: &crate::value_objects::FollowTarget,
    ) -> Result<(), DomainError> {
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
impl super::FollowGraphQuery for NoopSocialQuery {
    async fn get_following(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_followers(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_followers(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_following(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_followers(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_pending_followers(&self, _: &UserId) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_relation(
        &self,
        _: &UserId,
        _: &SocialIdentity,
    ) -> Result<FollowRelation, DomainError> {
        Ok(FollowRelation::default())
    }
}

#[async_trait]
impl super::BlockQuery for NoopSocialQuery {
    async fn get_blocked(&self, _: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
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

// ── NoopApDocument ───────────────────────────────────────────────────────────

/// Stub used when federation is disabled — every operation is a no-op.
pub struct NoopApDocument;

#[async_trait]
impl super::ApDocumentPort for NoopApDocument {
    async fn actor_json(&self, _: &str) -> Result<String, DomainError> {
        Ok(String::new())
    }
    async fn followers_collection_json(
        &self,
        _: uuid::Uuid,
        _: Option<u32>,
    ) -> Result<String, DomainError> {
        Ok(String::new())
    }
    async fn following_collection_json(
        &self,
        _: uuid::Uuid,
        _: Option<u32>,
    ) -> Result<String, DomainError> {
        Ok(String::new())
    }
}

// ── NoopInstanceBlocklist ────────────────────────────────────────────────────

/// Stub used when federation is disabled — every operation is a no-op.
pub struct NoopInstanceBlocklist;

#[async_trait]
impl super::InstanceBlocklistPort for NoopInstanceBlocklist {
    async fn get_blocked_domains(
        &self,
    ) -> Result<Vec<crate::models::BlockedDomainInfo>, DomainError> {
        Ok(vec![])
    }
    async fn add_blocked_domain(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_blocked_domain(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── NoopApBackfill ───────────────────────────────────────────────────────────

/// Stub used when federation is disabled — every operation is a no-op.
pub struct NoopApBackfill;

#[async_trait]
impl super::ApBackfillPort for NoopApBackfill {
    async fn get_following(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::models::FollowedActorInfo>, DomainError> {
        Ok(vec![])
    }
    async fn import_remote_outbox(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn run_backfill_for_follower(&self, _: uuid::Uuid, _: String) -> Result<(), DomainError> {
        Ok(())
    }
}
