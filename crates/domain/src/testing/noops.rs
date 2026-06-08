use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, ObjectStorage},
    value_objects::UserId,
};

// ── NoopEventPublisher ────────────────────────────────────────────────────────

pub struct NoopEventPublisher {
    pub events: Mutex<Vec<DomainEvent>>,
}

impl NoopEventPublisher {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(vec![]),
        })
    }

    pub fn published(&self) -> Vec<DomainEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

// ── NoopObjectStorage ──────────────────────────────────────────────────────────

pub struct NoopObjectStorage;

#[async_trait]
impl ObjectStorage for NoopObjectStorage {
    async fn store(&self, key: &str, _image_bytes: &[u8]) -> Result<String, DomainError> {
        Ok(format!("noop://{key}"))
    }

    async fn get(&self, _key: &str) -> Result<Vec<u8>, DomainError> {
        Ok(vec![])
    }

    async fn get_stream(
        &self,
        _key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn delete(&self, _key: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── NoopRemoteWatchlistRepository ─────────────────────────────────────────────

pub struct NoopRemoteWatchlistRepository;

#[async_trait]
impl crate::ports::RemoteWatchlistRepository for NoopRemoteWatchlistRepository {
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

pub struct NoopSocialQueryPort;

#[async_trait]
impl crate::ports::SocialQueryPort for NoopSocialQueryPort {
    async fn get_accepted_following_urls(&self, _: uuid::Uuid) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<crate::ports::RemoteActorInfo>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_accepted_followers(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_pending_followers(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::ports::PendingFollowerInfo>, DomainError> {
        Ok(vec![])
    }
}

// ── NoopGoalRepository ────────────────────────────────────────────────────────

pub struct NoopGoalRepository;

#[async_trait]
impl crate::ports::GoalRepository for NoopGoalRepository {
    async fn save(&self, _: &crate::models::Goal) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update(&self, _: &crate::models::Goal) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete(
        &self,
        _: &crate::value_objects::GoalId,
        _: &UserId,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_by_user_and_year(
        &self,
        _: &UserId,
        _: u16,
    ) -> Result<Option<crate::models::Goal>, DomainError> {
        Ok(None)
    }
    async fn list_for_user(&self, _: &UserId) -> Result<Vec<crate::models::Goal>, DomainError> {
        Ok(vec![])
    }
    async fn count_reviews_in_year(&self, _: &UserId, _: u16) -> Result<u32, DomainError> {
        Ok(0)
    }
}

// ── NoopUserSettingsRepository ────────────────────────────────────────────────

pub struct NoopUserSettingsRepository;

#[async_trait]
impl crate::ports::UserSettingsRepository for NoopUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<crate::models::UserSettings, DomainError> {
        Ok(crate::models::UserSettings::new(user_id.clone()))
    }
    async fn save(&self, _: &crate::models::UserSettings) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── NoopRemoteGoalRepository ──────────────────────────────────────────────────

pub struct NoopRemoteGoalRepository;

#[async_trait]
impl crate::ports::RemoteGoalRepository for NoopRemoteGoalRepository {
    async fn save(&self, _: crate::models::RemoteGoalEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_by_ap_id(&self, _: &str, _: u32, _: u32) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteGoalEntry>, DomainError> {
        Ok(vec![])
    }
}
