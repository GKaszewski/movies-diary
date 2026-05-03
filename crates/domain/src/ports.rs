use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{DiaryEntry, DiaryFilter, Movie, Review, ReviewHistory, collections::Paginated},
    value_objects::{ExternalMetadataId, MovieId, PasswordHash, PosterPath, UserId},
};

#[async_trait]
pub trait MovieRepository: Send + Sync {
    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError>;

    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError>;

    async fn query_diary(&self, filter: &DiaryFilter)
    -> Result<Paginated<DiaryEntry>, DomainError>;

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError>;
}

#[async_trait]
pub trait MetadataClient: Send + Sync {
    async fn fetch_movie_metadata(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Movie, DomainError>;
}

#[async_trait]
pub trait PosterFetcherClient: Send + Sync {
    async fn fetch_poster_bytes(&self, poster_url: &str) -> Result<Vec<u8>, DomainError>;
}

#[async_trait]
pub trait PosterStorage: Send + Sync {
    async fn store_poster(
        &self,
        movie_id: &MovieId,
        image_bytes: &[u8],
    ) -> Result<PosterPath, DomainError>;

    async fn get_poster(&self, poster_path: &PosterPath) -> Result<Vec<u8>, DomainError>;
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<UserId, DomainError>;
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plain_password: &str) -> Result<PasswordHash, DomainError>;

    async fn verify(&self, plain_password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}
