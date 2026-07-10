use async_trait::async_trait;

use crate::errors::DomainError;

#[async_trait]
pub trait ImageFetcher: Send + Sync {
    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, DomainError>;
}
