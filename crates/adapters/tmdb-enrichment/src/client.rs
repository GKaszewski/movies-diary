use domain::errors::DomainError;
use serde::Deserialize;

pub struct TmdbEnrichmentClient {
    pub(crate) api_key: String,
    pub(crate) http: reqwest::Client,
}

impl TmdbEnrichmentClient {
    pub fn from_env() -> Result<Self, DomainError> {
        let api_key = std::env::var("TMDB_API_KEY")
            .map_err(|_| DomainError::InfrastructureError("TMDB_API_KEY is not set".into()))?;
        Ok(Self {
            api_key,
            http: reqwest::Client::new(),
        })
    }

    pub(crate) fn base(&self, path: &str) -> String {
        format!("https://api.themoviedb.org/3{}", path)
    }

    pub(crate) async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        extra: &[(&str, &str)],
    ) -> Result<T, DomainError> {
        let mut req = self
            .http
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

    pub(crate) async fn resolve_tmdb_id(&self, external_id: &str) -> Result<u64, DomainError> {
        if let Some(numeric) = external_id.strip_prefix("tmdb:") {
            return numeric.parse::<u64>().map_err(|_| {
                DomainError::InfrastructureError(format!("Invalid tmdb id: {numeric}"))
            });
        }

        #[derive(Deserialize)]
        struct FindResult {
            id: u64,
        }
        #[derive(Deserialize)]
        struct FindResponse {
            movie_results: Vec<FindResult>,
        }

        let url = self.base(&format!("/find/{}", external_id));
        let resp: FindResponse = self.get(&url, &[("external_source", "imdb_id")]).await?;
        resp.movie_results
            .into_iter()
            .next()
            .map(|r| r.id)
            .ok_or_else(|| DomainError::NotFound(format!("TMDb: no movie for {external_id}")))
    }
}
