use async_trait::async_trait;
use chrono::Utc;

use crate::{
    errors::DomainError, models::RefreshSession, ports::RefreshSessionRepository,
    value_objects::UserId,
};

use super::InMemoryStore;

#[async_trait]
impl RefreshSessionRepository for InMemoryStore {
    async fn create(&self, session: &RefreshSession) -> Result<(), DomainError> {
        let mut sessions = self.refresh_sessions.write().unwrap();
        if sessions.iter().any(|s| s.token == session.token) {
            return Err(DomainError::InfrastructureError(
                "Database operation failed".into(),
            ));
        }
        sessions.push(session.clone());
        Ok(())
    }

    async fn get_by_token(&self, token: &str) -> Result<Option<RefreshSession>, DomainError> {
        Ok(self
            .refresh_sessions
            .read()
            .unwrap()
            .iter()
            .find(|s| s.token == token)
            .cloned())
    }

    async fn revoke(&self, token: &str) -> Result<(), DomainError> {
        self.refresh_sessions
            .write()
            .unwrap()
            .retain(|s| s.token != token);
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        self.refresh_sessions
            .write()
            .unwrap()
            .retain(|s| s.user_id != *user_id);
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let mut store = self.refresh_sessions.write().unwrap();
        let before = store.len();
        let now = Utc::now();
        store.retain(|s| s.expires_at >= now);
        Ok((before - store.len()) as u64)
    }
}
