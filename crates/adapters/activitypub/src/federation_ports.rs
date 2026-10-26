use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{BlockedDomainInfo, FollowedActorInfo},
    ports::{ApBackfillPort, ApDocumentPort, InstanceBlocklistPort, UserRepository},
    value_objects::UserId,
};
use k_ap::ActivityPubService;
use uuid::Uuid;

pub struct ApServiceAdapter {
    service: Arc<ActivityPubService>,
    users: Arc<dyn UserRepository>,
}

impl ApServiceAdapter {
    pub fn new(service: Arc<ActivityPubService>, users: Arc<dyn UserRepository>) -> Self {
        Self { service, users }
    }
}

fn ap_err(e: anyhow::Error) -> DomainError {
    tracing::error!("ActivityPub error: {:?}", e);
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl ApDocumentPort for ApServiceAdapter {
    async fn actor_is_gone(&self, user_id: Uuid) -> Result<bool, DomainError> {
        Ok(self
            .users
            .find_tombstoned_by_id(&UserId::from_uuid(user_id))
            .await?
            .is_some())
    }
    async fn actor_json(&self, user_id: &str) -> Result<String, DomainError> {
        self.service.actor_json(user_id).await.map_err(ap_err)
    }
    async fn followers_collection_json(
        &self,
        user_id: Uuid,
        page: Option<u32>,
    ) -> Result<String, DomainError> {
        self.service
            .followers_collection_json(user_id, page)
            .await
            .map_err(ap_err)
    }
    async fn following_collection_json(
        &self,
        user_id: Uuid,
        page: Option<u32>,
    ) -> Result<String, DomainError> {
        self.service
            .following_collection_json(user_id, page)
            .await
            .map_err(ap_err)
    }
}

#[async_trait]
impl InstanceBlocklistPort for ApServiceAdapter {
    async fn get_blocked_domains(&self) -> Result<Vec<BlockedDomainInfo>, DomainError> {
        let domains = self.service.get_blocked_domains().await.map_err(ap_err)?;
        Ok(domains
            .into_iter()
            .map(|d| BlockedDomainInfo {
                domain: d.domain,
                reason: d.reason,
                blocked_at: d.blocked_at,
            })
            .collect())
    }
    async fn add_blocked_domain(
        &self,
        domain: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError> {
        self.service
            .add_blocked_domain(domain, reason)
            .await
            .map_err(ap_err)
    }
    async fn remove_blocked_domain(&self, domain: &str) -> Result<(), DomainError> {
        self.service
            .remove_blocked_domain(domain)
            .await
            .map_err(ap_err)
    }
}

#[async_trait]
impl ApBackfillPort for ApServiceAdapter {
    async fn get_following(
        &self,
        local_user_id: Uuid,
    ) -> Result<Vec<FollowedActorInfo>, DomainError> {
        let actors = self
            .service
            .get_following(local_user_id)
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| FollowedActorInfo {
                url: a.url,
                outbox_url: a.outbox_url,
            })
            .collect())
    }
    async fn import_remote_outbox(
        &self,
        outbox_url: &str,
        actor_url: &str,
    ) -> Result<(), DomainError> {
        self.service
            .import_remote_outbox(outbox_url, actor_url)
            .await
            .map_err(ap_err)
    }
    async fn run_backfill_for_follower(
        &self,
        owner_user_id: Uuid,
        follower_inbox_url: String,
    ) -> Result<(), DomainError> {
        self.service
            .run_backfill_for_follower(owner_user_id, follower_inbox_url)
            .await
            .map_err(ap_err)
    }
}
