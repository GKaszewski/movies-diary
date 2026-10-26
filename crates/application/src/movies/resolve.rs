use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::Movie,
    ports::{MetadataClient, MovieCommand, MovieQuery},
    value_objects::MovieId,
};

use crate::diary::commands::MovieInput;
use crate::diary::movie_resolver::{MovieResolver, MovieResolverDeps};

pub async fn resolve_and_persist_movie(
    input: &MovieInput,
    movie_command: &dyn MovieCommand,
    movie_query: &dyn MovieQuery,
    metadata_client: &dyn MetadataClient,
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
        let events: Vec<DomainEvent> = movie
            .external_metadata_id()
            .map(|ext_id| DomainEvent::MovieDiscovered {
                movie_id: movie.id().clone(),
                external_metadata_id: ext_id.clone(),
            })
            .into_iter()
            .collect();
        movie_command.upsert_movie(&movie, &events).await?;
    }

    Ok((movie, is_new))
}
