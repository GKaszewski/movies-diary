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

// Re-export production noop types so test code that imports from
// `domain::testing` keeps compiling without changes.
pub use crate::ports::noop::NoopFederationAdminQuery;
pub use crate::ports::noop::NoopRemoteWatchlistRepository;

// ── NoopGoalCommand ───────────────────────────────────────────────────────────

pub struct NoopGoalCommand;

#[async_trait]
impl crate::ports::GoalCommand for NoopGoalCommand {
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
}

// ── NoopGoalQuery ─────────────────────────────────────────────────────────────

pub struct NoopGoalQuery;

#[async_trait]
impl crate::ports::GoalQuery for NoopGoalQuery {
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
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteGoalEntry>, DomainError> {
        Ok(vec![])
    }
}
