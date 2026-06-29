use async_trait::async_trait;
use domain::{errors::DomainError, models::PersonEnrichmentData, ports::PersonEnrichmentClient};
use serde::Deserialize;

use crate::client::TmdbEnrichmentClient;

#[async_trait]
impl PersonEnrichmentClient for TmdbEnrichmentClient {
    async fn fetch_details(&self, external_id: &str) -> Result<PersonEnrichmentData, DomainError> {
        let tmdb_id = external_id
            .strip_prefix("tmdb:")
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| {
                DomainError::InfrastructureError(format!(
                    "Cannot parse person external_id: {external_id}"
                ))
            })?;

        #[derive(Deserialize)]
        struct PersonDetails {
            biography: Option<String>,
            birthday: Option<String>,
            deathday: Option<String>,
            place_of_birth: Option<String>,
            also_known_as: Option<Vec<String>>,
            homepage: Option<String>,
            imdb_id: Option<String>,
        }

        let url = self.base(&format!("/person/{tmdb_id}"));
        let d: PersonDetails = self.get(&url, &[]).await?;

        Ok(PersonEnrichmentData {
            biography: d.biography.filter(|s| !s.is_empty()),
            birthday: d
                .birthday
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            deathday: d
                .deathday
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            place_of_birth: d.place_of_birth.filter(|s| !s.is_empty()),
            also_known_as: d.also_known_as.unwrap_or_default(),
            homepage: d.homepage.filter(|s| !s.is_empty()),
            imdb_id: d.imdb_id.filter(|s| !s.is_empty()),
        })
    }
}
