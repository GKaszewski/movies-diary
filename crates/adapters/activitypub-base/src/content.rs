use async_trait::async_trait;
use chrono::{DateTime, Utc};
use url::Url;

#[async_trait]
pub trait ApObjectHandler: Send + Sync {
    /// Returns (ap_id, serialized object) for all local content owned by this user.
    /// Used by outbox (count) and backfill (delivery). Must only return locally-authored content.
    async fn get_local_objects_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value)>>;

    /// Returns up to `limit` objects ordered newest-first, published before `before`.
    /// Returns (ap_id, object_json, published_at).
    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        before: Option<DateTime<Utc>>,
        limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, DateTime<Utc>)>>;

    /// Incoming Create activity — persist remote content.
    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()>;

    /// Incoming Update activity — update existing remote content.
    async fn on_update(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()>;

    /// Incoming Delete activity — remove specific remote content.
    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()>;

    /// Actor unfollowed/was removed — clean up all their remote content.
    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()>;
}
