use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    models::{BlockedDomainInfo, FollowedActorInfo},
};

/// Serves ActivityPub documents for content negotiation. Presentation calls
/// this when a peer sends `Accept: application/activity+json`.
#[async_trait]
pub trait ApDocumentPort: Send + Sync {
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

/// Instance-wide domain blocklist administration. Presentation calls this from
/// the admin API and the admin HTML pages; nothing outside presentation does.
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

/// Pulling remote content in, and pushing local content out, after a follow
/// is established. Worker-side only — no HTTP handler calls this.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::noop::{NoopApBackfill, NoopApDocument, NoopInstanceBlocklist};

    #[tokio::test]
    async fn noop_document_returns_empty_strings() {
        let p = NoopApDocument;
        assert_eq!(p.actor_json("anything").await.unwrap(), "");
        assert_eq!(
            p.followers_collection_json(uuid::Uuid::nil(), None)
                .await
                .unwrap(),
            ""
        );
        assert_eq!(
            p.following_collection_json(uuid::Uuid::nil(), Some(2))
                .await
                .unwrap(),
            ""
        );
    }

    #[tokio::test]
    async fn noop_blocklist_returns_empty_and_ok() {
        let p = NoopInstanceBlocklist;
        assert!(p.get_blocked_domains().await.unwrap().is_empty());
        p.add_blocked_domain("evil.example", Some("spam"))
            .await
            .unwrap();
        p.remove_blocked_domain("evil.example").await.unwrap();
    }

    #[tokio::test]
    async fn noop_backfill_returns_empty_and_ok() {
        let p = NoopApBackfill;
        assert!(p.get_following(uuid::Uuid::nil()).await.unwrap().is_empty());
        p.import_remote_outbox("https://a/outbox", "https://a")
            .await
            .unwrap();
        p.run_backfill_for_follower(uuid::Uuid::nil(), "https://a/inbox".into())
            .await
            .unwrap();
    }
}
