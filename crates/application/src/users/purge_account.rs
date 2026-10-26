use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{
        AccountPurger, EventHandler, FederationDeletionNotifier, ObjectStorage, UserRepository,
    },
};

pub struct AccountPurgeHandler {
    user: Arc<dyn UserRepository>,
    notifier: Arc<dyn FederationDeletionNotifier>,
    purger: Arc<dyn AccountPurger>,
    object_storage: Arc<dyn ObjectStorage>,
}

impl AccountPurgeHandler {
    pub fn new(
        user: Arc<dyn UserRepository>,
        notifier: Arc<dyn FederationDeletionNotifier>,
        purger: Arc<dyn AccountPurger>,
        object_storage: Arc<dyn ObjectStorage>,
    ) -> Self {
        Self {
            user,
            notifier,
            purger,
            object_storage,
        }
    }
}

#[async_trait]
impl EventHandler for AccountPurgeHandler {
    fn name(&self) -> &'static str {
        "account_purge"
    }

    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let DomainEvent::UserDeleted { user_id } = event else {
            return Ok(());
        };
        if self.user.find_tombstoned_by_id(user_id).await?.is_none() {
            return Ok(());
        }

        if !self.purger.is_purged(user_id).await?
            && let Err(e) = self.notifier.notify_deleted(user_id).await
        {
            tracing::warn!(user = %user_id.value(), "federation delete notice failed, purging anyway: {e}");
        }

        let keys = self.purger.owned_image_keys(user_id).await?;
        self.purger.purge(user_id).await?;
        for key in &keys {
            self.object_storage.delete(key).await?;
        }
        if !keys.is_empty() {
            self.purger.forget_images(user_id).await?;
        }
        Ok(())
    }
}
