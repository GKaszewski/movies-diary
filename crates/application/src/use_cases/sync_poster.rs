use domain::{
    errors::DomainError,
    value_objects::{ExternalMetadataId, MovieId},
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
        .poster_storage
        .store_poster(&movie_id, &image_bytes)
        .await?;

    movie.update_poster(stored_path);
    ctx.movie_repository.upsert_movie(&movie).await?;

    Ok(())
}
