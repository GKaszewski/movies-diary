use async_trait::async_trait;

use crate::{errors::DomainError, value_objects::UserId};

#[async_trait]
pub trait FederationDeletionNotifier: Send + Sync {
    async fn notify_deleted(&self, user: &UserId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait AccountPurger: Send + Sync {
    async fn is_purged(&self, user: &UserId) -> Result<bool, DomainError>;

    async fn owned_image_keys(&self, user: &UserId) -> Result<Vec<String>, DomainError>;

    async fn purge(&self, user: &UserId) -> Result<(), DomainError>;

    async fn forget_images(&self, user: &UserId) -> Result<(), DomainError>;

    async fn purge_expired_tombstones(
        &self,
        cutoff: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError>;
}
