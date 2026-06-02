use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::IndexableDocument,
    ports::{EventHandler, MovieRepository, SearchCommand},
};

/// Reacts to `MovieDiscovered` and inserts a bare search index entry immediately,
/// so movies are findable before TMDb enrichment runs.
/// Enrichment will later overwrite this with the full document (cast, genres, etc.).
pub struct MovieDiscoveryIndexer {
    movie_repository: Arc<dyn MovieRepository>,
    search_command: Arc<dyn SearchCommand>,
}

impl MovieDiscoveryIndexer {
    pub fn new(
        movie_repository: Arc<dyn MovieRepository>,
        search_command: Arc<dyn SearchCommand>,
    ) -> Self {
        Self {
            movie_repository,
            search_command,
        }
    }
}

#[async_trait]
impl EventHandler for MovieDiscoveryIndexer {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let movie_id = match event {
            DomainEvent::MovieDiscovered { movie_id, .. } => movie_id.clone(),
            _ => return Ok(()),
        };

        let Some(movie) = self.movie_repository.get_movie_by_id(&movie_id).await? else {
            tracing::warn!(movie_id = %movie_id.value(), "MovieDiscoveryIndexer: movie not found");
            return Ok(());
        };

        if let Err(e) = self
            .search_command
            .index(IndexableDocument::Movie {
                id: movie_id.clone(),
                movie: Box::new(movie),
                profile: None,
            })
            .await
        {
            tracing::warn!(movie_id = %movie_id.value(), "failed to index movie on discovery: {e}");
        }

        Ok(())
    }
}
