use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{
        MetadataSearchCriteria, Movie, MovieFilter, MovieProfile, MovieSummary,
        collections::{PageParams, Paginated},
    },
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterUrl, ReleaseYear},
};

#[async_trait]
pub trait MovieRepository: Send + Sync {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError>;
    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError>;
    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError>;
    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError>;
    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError>;
    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError>;
    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError>;
    async fn list_movies(
        &self,
        page: &PageParams,
        filter: &MovieFilter,
    ) -> Result<Paginated<MovieSummary>, DomainError>;
    /// Returns all movies that have an external_metadata_id set. Used for deduplication.
    async fn list_movies_with_external_id(&self) -> Result<Vec<Movie>, DomainError>;
}

#[async_trait]
pub trait MetadataClient: Send + Sync {
    async fn fetch_movie_metadata(
        &self,
        criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError>;
    async fn get_poster_url(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError>;
}

#[async_trait]
pub trait MovieProfileRepository: Send + Sync {
    async fn upsert(&self, profile: &MovieProfile) -> Result<(), DomainError>;
    async fn get_by_movie_id(&self, id: &MovieId) -> Result<Option<MovieProfile>, DomainError>;
    /// Returns (movie_id, external_metadata_id) for movies with no profile or a stale one
    /// (enriched_at older than 30 days).
    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError>;
}

#[async_trait]
pub trait MovieEnrichmentClient: Send + Sync {
    /// Resolves an external ID (TMDb or IMDb) and fetches the full movie profile.
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError>;
}

#[async_trait]
pub trait MovieDeduplicator: Send + Sync {
    /// Atomically re-points all foreign keys (reviews, watchlist entries, movie profiles)
    /// from `old_id` to `canonical_id`, upserts the canonical movie record, then deletes
    /// the old duplicate. Returns the number of rows re-pointed across all tables.
    async fn merge_into_canonical(
        &self,
        old_id: &MovieId,
        canonical: &Movie,
    ) -> Result<u64, DomainError>;
}
