use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::IndexableDocument,
    value_objects::{MovieId, PosterPath},
};

use crate::{diary::commands::SyncPosterCommand, movies::deps::SyncPosterDeps};

pub async fn execute(deps: &SyncPosterDeps, cmd: SyncPosterCommand) -> Result<(), DomainError> {
    let movie_id = MovieId::from_uuid(cmd.movie_id);

    let mut movie = match deps.movie.get_movie_by_id(&movie_id).await? {
        Some(m) => m,
        None => {
            tracing::warn!(
                "Sync cancelled: Movie {} not found in local DB",
                movie_id.value()
            );
            return Err(DomainError::NotFound("Movie not found".into()));
        }
    };

    let external_metadata_id = movie
        .external_metadata_id()
        .ok_or_else(|| {
            DomainError::ValidationError(
                "Movie has no external metadata ID, cannot sync poster".into(),
            )
        })?
        .clone();

    let poster_url = match deps
        .metadata
        .get_poster_url(&external_metadata_id)
        .await
    {
        Ok(Some(url)) => url,
        Ok(None) => return Ok(()),
        Err(e) => {
            tracing::warn!("Warning: Failed to find poster URL metadata: {:?}", e);
            return Err(e);
        }
    };

    let image_bytes = deps
        .poster_fetcher
        .fetch_poster_bytes(&poster_url)
        .await?;

    let stored_path = deps
        .object_storage
        .store(&movie_id.value().to_string(), &image_bytes)
        .await?;

    if let Err(e) = deps
        .event_publisher
        .publish(&DomainEvent::ImageStored {
            key: stored_path.clone(),
        })
        .await
    {
        tracing::warn!("failed to emit ImageStored for {stored_path}: {e}");
    }

    let poster_path = PosterPath::new(stored_path)?;

    movie.update_poster(poster_path);
    deps.movie.upsert_movie(&movie).await?;

    // Refresh search index so the new poster_path is reflected immediately.
    // Fetch existing profile if available for a complete index document.
    let profile = deps
        .movie_profile
        .get_by_movie_id(&movie_id)
        .await
        .ok()
        .flatten();
    if let Err(e) = deps
        .search_command
        .index(IndexableDocument::Movie {
            id: movie_id.clone(),
            movie: Box::new(movie),
            profile: profile.map(Box::new),
        })
        .await
    {
        tracing::warn!(movie_id = %movie_id.value(), "failed to refresh search index after poster sync: {e}");
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/sync_poster.rs"]
mod tests;
