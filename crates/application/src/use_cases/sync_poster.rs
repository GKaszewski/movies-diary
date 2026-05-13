use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::IndexableDocument,
    value_objects::{ExternalMetadataId, MovieId, PosterPath},
};

use crate::{commands::SyncPosterCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: SyncPosterCommand) -> Result<(), DomainError> {
    let movie_id = MovieId::from_uuid(cmd.movie_id);
    let external_metadata_id = ExternalMetadataId::new(cmd.external_metadata_id)?;

    let mut movie = match ctx.movie_repository.get_movie_by_id(&movie_id).await? {
        Some(m) => m,
        None => {
            tracing::warn!(
                "Sync cancelled: Movie {} not found in local DB",
                movie_id.value()
            );
            return Err(DomainError::NotFound("Movie not found".into()));
        }
    };

    let poster_url = match ctx
        .metadata_client
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

    let image_bytes = ctx.poster_fetcher.fetch_poster_bytes(&poster_url).await?;

    let stored_path = ctx
        .image_storage
        .store(&movie_id.value().to_string(), &image_bytes)
        .await?;

    if let Err(e) = ctx
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
    ctx.movie_repository.upsert_movie(&movie).await?;

    // Refresh search index so the new poster_path is reflected immediately.
    // Fetch existing profile if available for a complete index document.
    let profile = ctx
        .movie_profile_repository
        .get_by_movie_id(&movie_id)
        .await
        .ok()
        .flatten();
    if let Err(e) = ctx
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
