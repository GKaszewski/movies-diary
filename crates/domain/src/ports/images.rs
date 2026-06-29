use async_trait::async_trait;

use crate::{errors::DomainError, value_objects::PosterUrl};

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// Stores `image_bytes` at `key` and returns the stored key.
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError>;
    async fn get_stream(
        &self,
        key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>;
    async fn delete(&self, key: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PosterFetcherClient: Send + Sync {
    async fn fetch_poster_bytes(&self, poster_url: &PosterUrl) -> Result<Vec<u8>, DomainError>;
}

#[async_trait]
pub trait ImageRefCommand: Send + Sync {
    async fn swap(&self, old_key: &str, new_key: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImageRefQuery: Send + Sync {
    async fn list_keys(&self) -> Result<Vec<String>, DomainError>;
}
