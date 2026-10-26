use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    models::{BlockedDomainInfo, FollowedActorInfo},
};

#[async_trait]
pub trait ApDocumentPort: Send + Sync {
    async fn actor_is_gone(&self, _user_id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn actor_json(&self, user_id: &str) -> Result<String, DomainError>;
    async fn followers_collection_json(
        &self,
        user_id: Uuid,
        page: Option<u32>,
    ) -> Result<String, DomainError>;
    async fn following_collection_json(
        &self,
        user_id: Uuid,
        page: Option<u32>,
    ) -> Result<String, DomainError>;
}

#[async_trait]
pub trait InstanceBlocklistPort: Send + Sync {
    async fn get_blocked_domains(&self) -> Result<Vec<BlockedDomainInfo>, DomainError>;
    async fn add_blocked_domain(
        &self,
        domain: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn remove_blocked_domain(&self, domain: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ApBackfillPort: Send + Sync {
    async fn get_following(
        &self,
        local_user_id: Uuid,
    ) -> Result<Vec<FollowedActorInfo>, DomainError>;
    async fn import_remote_outbox(
        &self,
        outbox_url: &str,
        actor_url: &str,
    ) -> Result<(), DomainError>;
    async fn run_backfill_for_follower(
        &self,
        owner_user_id: Uuid,
        follower_inbox_url: String,
    ) -> Result<(), DomainError>;
}
