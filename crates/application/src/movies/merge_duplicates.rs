use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::{MovieDeduplicator, MovieQuery, ObjectStorage},
    value_objects::MovieId,
};

pub struct MergeDuplicatesDeps {
    pub movie_query: Arc<dyn MovieQuery>,
    pub deduplicator: Arc<dyn MovieDeduplicator>,
    pub object_storage: Arc<dyn ObjectStorage>,
}

pub struct MergeReport {
    pub pairs_found: u64,
    pub rows_repointed: u64,
}

pub async fn execute(deps: &MergeDuplicatesDeps) -> Result<MergeReport, DomainError> {
    let movies = deps.movie_query.list_movies_with_external_id().await?;

    let mut pairs_found = 0u64;
    let mut rows_repointed = 0u64;

    for movie in movies {
        let external_id = match movie.external_metadata_id() {
            Some(id) => id,
            None => continue,
        };

        let canonical_id = MovieId::from_external(external_id);
        if movie.id() == &canonical_id {
            continue; // already canonical
        }

        pairs_found += 1;

        // Determine which poster will be dropped after merge
        let canonical = match deps.movie_query.get_movie_by_id(&canonical_id).await? {
            Some(existing) => existing,
            None => domain::models::Movie::from_persistence(
                canonical_id,
                movie.external_metadata_id().cloned(),
                movie.title().clone(),
                movie.release_year().clone(),
                movie.director().map(str::to_string),
                movie.poster_path().cloned(),
            ),
        };

        // The COALESCE in merge_into_canonical keeps canonical's poster if it has one,
        // otherwise takes old's. Work out which poster key will be orphaned.
        let orphaned_poster = match (canonical.poster_path(), movie.poster_path()) {
            (Some(_), Some(old_poster)) if canonical.poster_path() != movie.poster_path() => {
                // Canonical wins — old movie's poster will be orphaned
                Some(old_poster.value().to_string())
            }
            (None, Some(_)) => None, // old poster moves to canonical, nothing orphaned
            _ => None,
        };

        let repointed = deps
            .deduplicator
            .merge_into_canonical(movie.id(), &canonical)
            .await?;

        // Delete the orphaned poster file from object storage
        if let Some(key) = orphaned_poster
            && let Err(e) = deps.object_storage.delete(&key).await
        {
            tracing::warn!(key, "failed to delete orphaned poster: {e}");
        }

        rows_repointed += repointed;

        tracing::info!(
            old_id = %movie.id().value(),
            canonical_id = %canonical.id().value(),
            external_id = %external_id.value(),
            rows_repointed = repointed,
            "merged duplicate movie"
        );
    }

    Ok(MergeReport {
        pairs_found,
        rows_repointed,
    })
}
