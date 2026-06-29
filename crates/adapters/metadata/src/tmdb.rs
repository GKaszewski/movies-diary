use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::MetadataSearchCriteria,
    value_objects::{ExternalMetadataId, MovieTitle, PosterUrl, ReleaseYear},
};
use serde::Deserialize;

use crate::{MetadataProvider, ProviderMovie};

pub(crate) struct TmdbProvider {
    client: reqwest::Client,
    api_key: String,
}

impl TmdbProvider {
    pub(crate) fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    fn base(&self, path: &str) -> String {
        format!("https://api.themoviedb.org/3{}", path)
    }

    fn poster_url(&self, path: &str) -> Option<PosterUrl> {
        if path.is_empty() || path == "null" {
            return None;
        }
        PosterUrl::new(format!("https://image.tmdb.org/t/p/w500{}", path)).ok()
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        extra: &[(&str, &str)],
    ) -> Result<T, DomainError> {
        let mut req = self
            .client
            .get(url)
            .query(&[("api_key", self.api_key.as_str())]);
        for (k, v) in extra {
            req = req.query(&[(k, v)]);
        }
        req.send()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .error_for_status()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .json::<T>()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn fetch_details(&self, tmdb_id: u64) -> Result<ProviderMovie, DomainError> {
        #[derive(Deserialize)]
        struct CrewMember {
            job: String,
            name: String,
        }
        #[derive(Deserialize)]
        struct Credits {
            crew: Vec<CrewMember>,
        }
        #[derive(Deserialize)]
        struct Details {
            imdb_id: Option<String>,
            title: String,
            release_date: String, // "YYYY-MM-DD"
            poster_path: Option<String>,
            credits: Credits,
        }

        let url = self.base(&format!("/movie/{}", tmdb_id));
        let d: Details = self.get(&url, &[("append_to_response", "credits")]).await?;

        let year: u16 = d
            .release_date
            .split('-')
            .next()
            .and_then(|y| y.parse().ok())
            .ok_or_else(|| {
                DomainError::InfrastructureError(format!(
                    "Unparseable release_date: {}",
                    d.release_date
                ))
            })?;

        // Prefer IMDB ID; fall back to "tmdb:{id}" so the record is still usable.
        let raw_id = d
            .imdb_id
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("tmdb:{}", tmdb_id));

        let imdb_id = ExternalMetadataId::new(raw_id)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let title = MovieTitle::new(d.title)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let release_year =
            ReleaseYear::new(year).map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let director = d
            .credits
            .crew
            .into_iter()
            .find(|c| c.job == "Director")
            .map(|c| c.name);

        let poster_url = d.poster_path.as_deref().and_then(|p| self.poster_url(p));

        Ok(ProviderMovie {
            imdb_id,
            title,
            release_year,
            director,
            poster_url,
        })
    }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
    async fn fetch(&self, criteria: &MetadataSearchCriteria) -> Result<ProviderMovie, DomainError> {
        let tmdb_id: u64 = match criteria {
            MetadataSearchCriteria::ImdbId(id) => {
                #[derive(Deserialize)]
                struct FindResult {
                    id: u64,
                }
                #[derive(Deserialize)]
                struct FindResponse {
                    movie_results: Vec<FindResult>,
                }
                let url = self.base(&format!("/find/{}", id.value()));
                let resp: FindResponse = self.get(&url, &[("external_source", "imdb_id")]).await?;
                resp.movie_results
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        DomainError::NotFound(format!("TMDB: no movie for {}", id.value()))
                    })?
                    .id
            }
            MetadataSearchCriteria::Title { title, year } => {
                #[derive(Deserialize)]
                struct SearchResult {
                    id: u64,
                }
                #[derive(Deserialize)]
                struct SearchResponse {
                    results: Vec<SearchResult>,
                }
                let url = self.base("/search/movie");
                let mut extra = vec![("query", title.value())];
                let year_str;
                if let Some(y) = year {
                    year_str = y.value().to_string();
                    extra.push(("year", year_str.as_str()));
                }
                let resp: SearchResponse = self.get(&url, &extra).await?;
                resp.results
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        DomainError::NotFound(format!("TMDB: no results for '{}'", title.value()))
                    })?
                    .id
            }
        };

        self.fetch_details(tmdb_id).await
    }
}
