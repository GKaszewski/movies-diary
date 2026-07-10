use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::Movie,
    ports::{EventPublisher, MetadataClient, MovieCommand, MovieQuery},
    value_objects::MovieId,
};

use crate::diary::commands::MovieInput;
use crate::diary::movie_resolver::{MovieResolver, MovieResolverDeps};

/// Resolves a movie from input, persists it, and publishes `MovieDiscovered` if new.
///
/// Returns `(movie, is_new_movie)`.
pub async fn resolve_and_persist_movie(
    input: &MovieInput,
    movie_command: &dyn MovieCommand,
    movie_query: &dyn MovieQuery,
    metadata_client: &dyn MetadataClient,
    event_publisher: &dyn EventPublisher,
) -> Result<(Movie, bool), DomainError> {
    let (movie, is_new) = if let Some(id) = input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        let movie = movie_query
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?;
        (movie, false)
    } else {
        let deps = MovieResolverDeps {
            repository: movie_query,
            metadata_client,
        };
        MovieResolver::default_pipeline()
            .resolve(input, &deps)
            .await?
    };

    if is_new {
        movie_command.upsert_movie(&movie).await?;
        if let Some(ext_id) = movie.external_metadata_id() {
            let _ = event_publisher
                .publish(&DomainEvent::MovieDiscovered {
                    movie_id: movie.id().clone(),
                    external_metadata_id: ext_id.clone(),
                })
                .await;
        }
    }

    Ok((movie, is_new))
}
