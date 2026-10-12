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
pub trait MovieCommand: Send + Sync {
    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError>;
    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait MovieQuery: Send + Sync {
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
    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError>;
}

#[async_trait]
pub trait MovieEnrichmentClient: Send + Sync {
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError>;
}

#[async_trait]
pub trait MovieDeduplicator: Send + Sync {
    async fn merge_into_canonical(
        &self,
        old_id: &MovieId,
        canonical: &Movie,
    ) -> Result<u64, DomainError>;
}
