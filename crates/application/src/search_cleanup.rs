use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::EntityType,
    ports::{EventHandler, SearchCommand},
};

pub struct SearchCleanupHandler {
    search_command: Arc<dyn SearchCommand>,
}

impl SearchCleanupHandler {
    pub fn new(search_command: Arc<dyn SearchCommand>) -> Self {
        Self { search_command }
    }
}

#[async_trait]
impl EventHandler for SearchCleanupHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let movie_id = match event {
            DomainEvent::MovieDeleted { movie_id, .. } => movie_id.value().to_string(),
            _ => return Ok(()),
        };

        if let Err(e) = self.search_command.remove(EntityType::Movie, &movie_id).await {
            tracing::warn!("search cleanup failed for movie {movie_id}: {e}");
        }
        Ok(())
    }
}
