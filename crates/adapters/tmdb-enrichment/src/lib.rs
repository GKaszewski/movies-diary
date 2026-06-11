use std::sync::Arc;

use application::movies::{commands::EnrichMovieCommand, enrich_movie, request_enrichment};
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{CastMember, CrewMember, Genre, Keyword, MovieProfile, PersonEnrichmentData},
    ports::{
        EventHandler, MovieEnrichmentClient, MovieProfileRepository, MovieRepository,
        ObjectStorage, PersonCommand, PersonEnrichmentClient, PersonQuery, SearchCommand,
    },
    value_objects::MovieId,
};
use serde::Deserialize;

// ── TMDb enrichment client ───────────────────────────────────────────────────

pub struct TmdbEnrichmentClient {
    api_key: String,
    http: reqwest::Client,
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

    fn base(&self, path: &str) -> String {
        format!("https://api.themoviedb.org/3{}", path)
    }

    async fn get<T: for<'de> Deserialize<'de>>(
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

    async fn resolve_tmdb_id(&self, external_id: &str) -> Result<u64, DomainError> {
        if let Some(numeric) = external_id.strip_prefix("tmdb:") {
            return numeric.parse::<u64>().map_err(|_| {
                DomainError::InfrastructureError(format!("Invalid tmdb id: {numeric}"))
            });
        }

        // Assume IMDb ID (tt…) — use /find
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

#[async_trait]
impl MovieEnrichmentClient for TmdbEnrichmentClient {
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError> {
        let tmdb_id = self.resolve_tmdb_id(external_metadata_id).await?;

        #[derive(Deserialize)]
        struct GenreDto {
            id: u32,
            name: String,
        }

        #[derive(Deserialize)]
        struct CollectionDto {
            name: String,
        }

        #[derive(Deserialize)]
        struct CastDto {
            id: u64,
            name: String,
            character: String,
            order: u32,
            profile_path: Option<String>,
        }

        #[derive(Deserialize)]
        struct CrewDto {
            id: u64,
            name: String,
            job: String,
            department: String,
            profile_path: Option<String>,
        }

        #[derive(Deserialize)]
        struct Credits {
            cast: Vec<CastDto>,
            crew: Vec<CrewDto>,
        }

        #[derive(Deserialize)]
        struct KeywordDto {
            id: u32,
            name: String,
        }

        #[derive(Deserialize)]
        struct Keywords {
            keywords: Vec<KeywordDto>,
        }

        #[derive(Deserialize)]
        struct Details {
            imdb_id: Option<String>,
            overview: Option<String>,
            tagline: Option<String>,
            runtime: Option<u32>,
            budget: Option<i64>,
            revenue: Option<i64>,
            vote_average: Option<f64>,
            vote_count: Option<u32>,
            original_language: Option<String>,
            genres: Vec<GenreDto>,
            belongs_to_collection: Option<CollectionDto>,
            credits: Credits,
            keywords: Keywords,
        }

        let url = self.base(&format!("/movie/{}", tmdb_id));
        let d: Details = self
            .get(&url, &[("append_to_response", "credits,keywords")])
            .await?;

        Ok(MovieProfile {
            movie_id,
            tmdb_id,
            imdb_id: d.imdb_id.filter(|s| !s.is_empty()),
            overview: d.overview.filter(|s| !s.is_empty()),
            tagline: d.tagline.filter(|s| !s.is_empty()),
            runtime_minutes: d.runtime,
            budget_usd: d.budget.filter(|&v| v > 0),
            revenue_usd: d.revenue.filter(|&v| v > 0),
            vote_average: d.vote_average,
            vote_count: d.vote_count,
            original_language: d.original_language,
            collection_name: d.belongs_to_collection.map(|c| c.name),
            genres: d
                .genres
                .into_iter()
                .map(|g| Genre {
                    tmdb_id: g.id,
                    name: g.name,
                })
                .collect(),
            keywords: d
                .keywords
                .keywords
                .into_iter()
                .map(|k| Keyword {
                    tmdb_id: k.id,
                    name: k.name,
                })
                .collect(),
            cast: d
                .credits
                .cast
                .into_iter()
                .map(|c| CastMember {
                    tmdb_person_id: c.id,
                    name: c.name,
                    character: c.character,
                    billing_order: c.order,
                    profile_path: c.profile_path,
                })
                .collect(),
            crew: d
                .credits
                .crew
                .into_iter()
                .map(|c| CrewMember {
                    tmdb_person_id: c.id,
                    name: c.name,
                    job: c.job,
                    department: c.department,
                    profile_path: c.profile_path,
                })
                .collect(),
            enriched_at: Utc::now(),
        })
    }
}

// ── Person enrichment client ────────────────────────────────────────────────

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

// ── Movie enrichment event handler ──────────────────────────────────────────

pub struct EnrichmentHandler {
    pub enrichment_client: Arc<dyn MovieEnrichmentClient>,
    pub movie_repository: Arc<dyn MovieRepository>,
    pub profile_repo: Arc<dyn MovieProfileRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub search_command: Arc<dyn SearchCommand>,
    pub object_storage: Arc<dyn ObjectStorage>,
    http: reqwest::Client,
}

impl EnrichmentHandler {
    pub fn new(
        enrichment_client: Arc<dyn MovieEnrichmentClient>,
        movie_repository: Arc<dyn MovieRepository>,
        profile_repo: Arc<dyn MovieProfileRepository>,
        person_command: Arc<dyn PersonCommand>,
        search_command: Arc<dyn SearchCommand>,
        object_storage: Arc<dyn ObjectStorage>,
    ) -> Self {
        Self {
            enrichment_client,
            movie_repository,
            profile_repo,
            person_command,
            search_command,
            object_storage,
            http: reqwest::Client::new(),
        }
    }

    async fn download_cast_photos(&self, profile: &MovieProfile) {
        for member in profile.cast.iter().take(5) {
            let Some(ref path) = member.profile_path else {
                continue;
            };
            let key = format!("cast{path}");
            if self.object_storage.get(&key).await.is_ok() {
                continue;
            }
            let url = format!("https://image.tmdb.org/t/p/w185{path}");
            match self.http.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await
                        && let Err(e) = self.object_storage.store(&key, &bytes).await
                    {
                        tracing::debug!("cast photo store failed for {path}: {e}");
                    }
                }
                _ => tracing::debug!("cast photo download failed for {path}"),
            }
        }
    }
}

#[async_trait]
impl EventHandler for EnrichmentHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (movie_id, external_metadata_id) = match event {
            DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            } => (movie_id.clone(), external_metadata_id.clone()),
            _ => return Ok(()),
        };

        let Some(profile) = request_enrichment::fetch_if_stale(
            self.enrichment_client.as_ref(),
            &self.profile_repo,
            movie_id.clone(),
            &external_metadata_id,
        )
        .await?
        else {
            return Ok(());
        };

        self.download_cast_photos(&profile).await;
        enrich_movie::execute(
            &self.movie_repository,
            &self.profile_repo,
            &self.person_command,
            &self.search_command,
            EnrichMovieCommand { movie_id, profile },
        )
        .await
    }
}

// ── Person enrichment event handler ─────────────────────────────────────────

pub struct PersonEnrichmentHandler {
    enrichment_client: Arc<dyn PersonEnrichmentClient>,
    person_query: Arc<dyn PersonQuery>,
    person_command: Arc<dyn PersonCommand>,
}

impl PersonEnrichmentHandler {
    pub fn new(
        enrichment_client: Arc<dyn PersonEnrichmentClient>,
        person_query: Arc<dyn PersonQuery>,
        person_command: Arc<dyn PersonCommand>,
    ) -> Self {
        Self {
            enrichment_client,
            person_query,
            person_command,
        }
    }
}

const PERSON_STALENESS_DAYS: i64 = 90;

#[async_trait]
impl EventHandler for PersonEnrichmentHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (person_id, external_person_id) = match event {
            DomainEvent::PersonEnrichmentRequested {
                person_id,
                external_person_id,
            } => (person_id.clone(), external_person_id.clone()),
            _ => return Ok(()),
        };

        if let Some(person) = self.person_query.get_by_id(&person_id).await? {
            if let Some(at) = person.enriched_at() {
                if (Utc::now() - at).num_days() < PERSON_STALENESS_DAYS {
                    tracing::debug!(person_id = %person_id.value(), "person enrichment still fresh");
                    return Ok(());
                }
            }
        }

        tracing::info!(person_id = %person_id.value(), "enriching person from TMDb");
        let data = self
            .enrichment_client
            .fetch_details(&external_person_id)
            .await?;
        self.person_command
            .update_enrichment(&person_id, &data)
            .await
    }
}
