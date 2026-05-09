use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::MetadataSearchCriteria,
    value_objects::{ExternalMetadataId, MovieTitle, PosterUrl, ReleaseYear},
};
use serde::Deserialize;

use crate::{MetadataProvider, ProviderMovie};

pub(crate) struct OmdbProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OmdbProvider {
    pub(crate) fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "http://www.omdbapi.com/".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct OmdbResponse {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Year")]
    year: String,
    #[serde(rename = "Director")]
    director: String,
    #[serde(rename = "Poster")]
    poster: String,
    #[serde(rename = "imdbID")]
    imdb_id: String,
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "Error")]
    error: Option<String>,
}

#[async_trait]
impl MetadataProvider for OmdbProvider {
    async fn fetch(&self, criteria: &MetadataSearchCriteria) -> Result<ProviderMovie, DomainError> {
        let mut url = reqwest::Url::parse(&self.base_url)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        {
            let mut params = url.query_pairs_mut();
            params.append_pair("apikey", &self.api_key);
            match criteria {
                MetadataSearchCriteria::ImdbId(id) => {
                    params.append_pair("i", id.value());
                }
                MetadataSearchCriteria::Title { title, year } => {
                    params.append_pair("t", title.value());
                    if let Some(y) = year {
                        params.append_pair("y", &y.value().to_string());
                    }
                }
            }
        }

        let http_resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e: reqwest::Error| DomainError::InfrastructureError(e.to_string()))?
            .error_for_status()
            .map_err(|e: reqwest::Error| DomainError::InfrastructureError(e.to_string()))?;

        let resp: OmdbResponse = http_resp
            .json()
            .await
            .map_err(|e: reqwest::Error| DomainError::InfrastructureError(e.to_string()))?;

        if resp.response != "True" {
            let msg = resp.error.unwrap_or_default();
            return if msg.to_lowercase().contains("not found") {
                Err(DomainError::NotFound(msg))
            } else {
                Err(DomainError::InfrastructureError(msg))
            };
        }

        let year: u16 = resp
            .year
            .chars()
            .take(4)
            .collect::<String>()
            .parse()
            .map_err(|_| {
                DomainError::InfrastructureError(format!("Unparseable year: {}", resp.year))
            })?;

        let imdb_id = ExternalMetadataId::new(resp.imdb_id)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let title = MovieTitle::new(resp.title)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let release_year =
            ReleaseYear::new(year).map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let director = match resp.director.as_str() {
            "N/A" | "" => None,
            d => Some(d.to_string()),
        };

        let poster_url = match resp.poster.as_str() {
            "N/A" | "" => None,
            url => PosterUrl::new(url.to_string()).ok(),
        };

        Ok(ProviderMovie {
            imdb_id,
            title,
            release_year,
            director,
            poster_url,
        })
    }
}
