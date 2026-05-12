use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::EntityType,
    ports::{EventHandler, PersonQuery, SearchCommand},
};

pub struct SearchCleanupHandler {
    search_command: Arc<dyn SearchCommand>,
    person_query:   Arc<dyn PersonQuery>,
}

impl SearchCleanupHandler {
    pub fn new(search_command: Arc<dyn SearchCommand>, person_query: Arc<dyn PersonQuery>) -> Self {
        Self { search_command, person_query }
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

        // Remove persons who have no remaining movie credits (orphaned after cascade delete).
        match self.person_query.list_orphaned_persons().await {
            Ok(orphans) => {
                for person_id in orphans {
                    let id = person_id.value().to_string();
                    if let Err(e) = self.search_command.remove(EntityType::Person, &id).await {
                        tracing::warn!("search cleanup failed for orphaned person {id}: {e}");
                    }
                }
            }
            Err(e) => tracing::warn!("failed to list orphaned persons after movie {movie_id} deletion: {e}"),
        }

        Ok(())
    }
}
