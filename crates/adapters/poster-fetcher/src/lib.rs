mod config;
pub use config::PosterFetcherConfig;

use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{ImageFetcher, PosterFetcherClient},
    value_objects::PosterUrl,
};

pub struct ReqwestPosterFetcher {
    client: reqwest::Client,
}

impl ReqwestPosterFetcher {
    pub fn new(config: PosterFetcherConfig) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl PosterFetcherClient for ReqwestPosterFetcher {
    async fn fetch_poster_bytes(&self, poster_url: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        let bytes = self
            .client
            .get(poster_url.value())
            .send()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .error_for_status()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

#[async_trait]
impl ImageFetcher for ReqwestPosterFetcher {
    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, DomainError> {
        let bytes = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .error_for_status()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

pub fn create() -> anyhow::Result<std::sync::Arc<dyn domain::ports::PosterFetcherClient>> {
    Ok(std::sync::Arc::new(ReqwestPosterFetcher::new(
        PosterFetcherConfig::from_env(),
    )?))
}

pub fn create_image_fetcher() -> anyhow::Result<std::sync::Arc<dyn domain::ports::ImageFetcher>> {
    Ok(std::sync::Arc::new(ReqwestPosterFetcher::new(
        PosterFetcherConfig::from_env(),
    )?))
}
