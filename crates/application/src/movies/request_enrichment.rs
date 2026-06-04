use std::sync::Arc;

use chrono::Utc;
use domain::{
    errors::DomainError,
    models::MovieProfile,
    ports::{MovieEnrichmentClient, MovieProfileRepository},
    value_objects::MovieId,
};

const STALENESS_DAYS: i64 = 30;

pub async fn fetch_if_stale(
    enrichment_client: &dyn MovieEnrichmentClient,
    profile_repo: &Arc<dyn MovieProfileRepository>,
    movie_id: MovieId,
    external_metadata_id: &str,
) -> Result<Option<MovieProfile>, DomainError> {
    if let Ok(Some(existing)) = profile_repo.get_by_movie_id(&movie_id).await {
        let age = Utc::now() - existing.enriched_at;
        if age.num_days() < STALENESS_DAYS {
            tracing::debug!(
                movie_id = %movie_id.value(),
                "skipping enrichment — profile is {} days old",
                age.num_days()
            );
            return Ok(None);
        }
    }

    tracing::info!(movie_id = %movie_id.value(), external_id = %external_metadata_id, "enriching movie");

    match enrichment_client
        .fetch_profile(movie_id, external_metadata_id)
        .await
    {
        Ok(profile) => Ok(Some(profile)),
        Err(DomainError::NotFound(msg)) => {
            tracing::warn!("TMDb lookup found nothing: {msg}");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
