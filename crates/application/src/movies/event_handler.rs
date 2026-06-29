use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::MovieProfile,
    ports::{
        EventHandler, MovieEnrichmentClient, MovieProfileRepository, MovieRepository,
        ObjectStorage, PersonCommand, SearchCommand,
    },
};

use crate::movies::{
    commands::EnrichMovieCommand, deps::EnrichMovieDeps, enrich_movie, request_enrichment,
};

pub struct MovieEnrichmentHandler {
    enrichment_client: Arc<dyn MovieEnrichmentClient>,
    movie_repository: Arc<dyn MovieRepository>,
    profile_repo: Arc<dyn MovieProfileRepository>,
    person_command: Arc<dyn PersonCommand>,
    search_command: Arc<dyn SearchCommand>,
    object_storage: Arc<dyn ObjectStorage>,
    http: reqwest::Client,
}

impl MovieEnrichmentHandler {
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
impl EventHandler for MovieEnrichmentHandler {
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
            external_metadata_id.value(),
        )
        .await?
        else {
            return Ok(());
        };

        self.download_cast_photos(&profile).await;
        let enrich_deps = EnrichMovieDeps {
            movie: self.movie_repository.clone(),
            movie_profile: self.profile_repo.clone(),
            person_command: self.person_command.clone(),
            search_command: self.search_command.clone(),
        };
        enrich_movie::execute(&enrich_deps, EnrichMovieCommand { movie_id, profile }).await
    }
}
