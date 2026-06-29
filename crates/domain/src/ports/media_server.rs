use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{ParsedPlaybackEvent, WatchEvent, WatchEventStatus, WebhookToken},
    value_objects::{UserId, WatchEventId, WebhookTokenId},
};

pub trait MediaServerParser: Send + Sync {
    fn parse_playback_event(&self, body: &[u8])
    -> Result<Option<ParsedPlaybackEvent>, DomainError>;
}

#[async_trait]
pub trait WatchEventRepository: Send + Sync {
    async fn save(&self, event: &WatchEvent) -> Result<(), DomainError>;
    async fn update_status(
        &self,
        id: &WatchEventId,
        status: WatchEventStatus,
    ) -> Result<(), DomainError>;
    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError>;
    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError>;
    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError>;
    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        status: WatchEventStatus,
    ) -> Result<u64, DomainError>;
    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: NaiveDateTime,
    ) -> Result<bool, DomainError>;
    async fn delete_non_pending_older_than(
        &self,
        before: NaiveDateTime,
    ) -> Result<u64, DomainError>;
}

#[async_trait]
pub trait WebhookTokenRepository: Send + Sync {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError>;
    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError>;
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError>;
    async fn delete(&self, id: &WebhookTokenId, user_id: &UserId) -> Result<(), DomainError>;
    async fn touch_last_used(&self, id: &WebhookTokenId) -> Result<(), DomainError>;
}
