use async_trait::async_trait;
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{WatchEvent, WatchEventStatus, WebhookToken},
    ports::{WatchEventCommand, WatchEventQuery, WebhookTokenRepository},
    value_objects::{UserId, WatchEventId, WebhookTokenId},
};

use super::InMemoryStore;

#[async_trait]
impl WatchEventCommand for InMemoryStore {
    async fn save(&self, event: &WatchEvent, events: &[DomainEvent]) -> Result<(), DomainError> {
        self.watch_events.write().unwrap().push(event.clone());
        self.record_events(events);
        Ok(())
    }

    async fn update_status(
        &self,
        _id: &WatchEventId,
        _status: WatchEventStatus,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        _status: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        Ok(ids.len() as u64)
    }

    async fn delete_non_pending_older_than(
        &self,
        before: NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let mut store = self.watch_events.write().unwrap();
        let before_len = store.len();
        store.retain(|e| *e.status() == WatchEventStatus::Pending || *e.created_at() >= before);
        Ok((before_len - store.len()) as u64)
    }
}

#[async_trait]
impl WatchEventQuery for InMemoryStore {
    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError> {
        let store = self.watch_events.read().unwrap();
        Ok(store
            .iter()
            .filter(|e| {
                e.user_id().value() == user_id.value() && *e.status() == WatchEventStatus::Pending
            })
            .cloned()
            .collect())
    }

    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError> {
        let store = self.watch_events.read().unwrap();
        Ok(store.iter().find(|e| e.id().value() == id.value()).cloned())
    }

    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError> {
        let id_vals: Vec<Uuid> = ids.iter().map(|id| id.value()).collect();
        let store = self.watch_events.read().unwrap();
        Ok(store
            .iter()
            .filter(|e| id_vals.contains(&e.id().value()))
            .cloned()
            .collect())
    }

    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: NaiveDateTime,
    ) -> Result<bool, DomainError> {
        let store = self.watch_events.read().unwrap();
        Ok(store.iter().any(|e| {
            e.user_id().value() == user_id.value()
                && e.external_metadata_id() == Some(external_id)
                && *e.watched_at() > after
        }))
    }
}

#[async_trait]
impl WebhookTokenRepository for InMemoryStore {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError> {
        self.webhook_tokens.write().unwrap().push(token.clone());
        Ok(())
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError> {
        let store = self.webhook_tokens.read().unwrap();
        let gone = self.tombstoned.read().unwrap();
        Ok(store
            .iter()
            .find(|t| t.token_hash() == hash && !gone.contains(&t.user_id().value()))
            .cloned())
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError> {
        let store = self.webhook_tokens.read().unwrap();
        Ok(store
            .iter()
            .filter(|t| t.user_id().value() == user_id.value())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &WebhookTokenId, _user_id: &UserId) -> Result<(), DomainError> {
        self.webhook_tokens
            .write()
            .unwrap()
            .retain(|t| t.id().value() != id.value());
        Ok(())
    }

    async fn touch_last_used(&self, _id: &WebhookTokenId) -> Result<(), DomainError> {
        Ok(())
    }
}
