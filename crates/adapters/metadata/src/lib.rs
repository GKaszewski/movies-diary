use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{MetadataSearchCriteria, Movie},
    ports::MetadataClient,
    value_objects::{ExternalMetadataId, MovieTitle, PosterUrl, ReleaseYear},
};

mod omdb;
mod tmdb;

pub(crate) struct ProviderMovie {
    pub imdb_id: ExternalMetadataId,
    pub title: MovieTitle,
    pub release_year: ReleaseYear,
    pub director: Option<String>,
    pub poster_url: Option<PosterUrl>,
}

#[async_trait]
pub(crate) trait MetadataProvider: Send + Sync {
    async fn fetch(&self, criteria: &MetadataSearchCriteria) -> Result<ProviderMovie, DomainError>;
}

pub struct MetadataClientImpl {
    provider: Box<dyn MetadataProvider>,
}

impl MetadataClientImpl {
    pub fn new_omdb(api_key: String) -> Self {
        Self {
            provider: Box::new(omdb::OmdbProvider::new(api_key)),
        }
    }

    pub fn new_tmdb(api_key: String) -> Self {
        Self {
            provider: Box::new(tmdb::TmdbProvider::new(api_key)),
        }
    }
}

#[async_trait]
impl MetadataClient for MetadataClientImpl {
    async fn fetch_movie_metadata(
        &self,
        criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        let pm = self.provider.fetch(criteria).await?;
        Ok(Movie::new(
            Some(pm.imdb_id),
            pm.title,
            pm.release_year,
            pm.director,
            None,
        ))
    }

    async fn get_poster_url(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        let criteria = MetadataSearchCriteria::ImdbId(external_metadata_id.clone());
        let pm = self.provider.fetch(&criteria).await?;
        Ok(pm.poster_url)
    }
}

pub fn create() -> anyhow::Result<std::sync::Arc<dyn domain::ports::MetadataClient>> {
    use anyhow::Context;
    if let Ok(key) = std::env::var("TMDB_API_KEY") {
        Ok(std::sync::Arc::new(MetadataClientImpl::new_tmdb(key)))
    } else {
        let key = std::env::var("OMDB_API_KEY")
            .context("either TMDB_API_KEY or OMDB_API_KEY must be set")?;
        Ok(std::sync::Arc::new(MetadataClientImpl::new_omdb(key)))
    }
}
