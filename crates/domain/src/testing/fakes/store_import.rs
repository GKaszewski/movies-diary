use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{ImportProfile, ImportSession},
    ports::{ImportProfileRepository, ImportSessionRepository},
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

use super::InMemoryStore;

#[async_trait]
impl ImportSessionRepository for InMemoryStore {
    async fn create(&self, session: &ImportSession) -> Result<(), DomainError> {
        self.import_sessions
            .write()
            .unwrap()
            .insert(session.id.value(), session.clone());
        Ok(())
    }

    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        let store = self.import_sessions.read().unwrap();
        Ok(store
            .get(&id.value())
            .filter(|s| s.user_id.value() == user_id.value())
            .cloned())
    }

    async fn update(&self, session: &ImportSession) -> Result<(), DomainError> {
        self.import_sessions
            .write()
            .unwrap()
            .insert(session.id.value(), session.clone());
        Ok(())
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        self.import_sessions.write().unwrap().remove(&id.value());
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let mut store = self.import_sessions.write().unwrap();
        let now = chrono::Utc::now().naive_utc();
        let before_len = store.len();
        store.retain(|_, s| s.expires_at > now);
        Ok((before_len - store.len()) as u64)
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let mut store = self.import_sessions.write().unwrap();
        let now = chrono::Utc::now().naive_utc();
        store.retain(|_, s| !(s.user_id.value() == user_id.value() && s.expires_at <= now));
        Ok(())
    }
}

#[async_trait]
impl ImportProfileRepository for InMemoryStore {
    async fn save(&self, profile: &ImportProfile) -> Result<(), DomainError> {
        self.import_profiles
            .write()
            .unwrap()
            .insert(profile.id.value(), profile.clone());
        Ok(())
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        let store = self.import_profiles.read().unwrap();
        Ok(store
            .values()
            .filter(|p| p.user_id.value() == user_id.value())
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        id: &ImportProfileId,
        user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError> {
        let store = self.import_profiles.read().unwrap();
        Ok(store
            .get(&id.value())
            .filter(|p| p.user_id.value() == user_id.value())
            .cloned())
    }

    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError> {
        self.import_profiles.write().unwrap().remove(&id.value());
        Ok(())
    }
}
