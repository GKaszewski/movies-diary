use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, MetadataClient, MovieRepository, PosterFetcherClient, PosterStorage},
    value_objects::{ExternalMetadataId, MovieId},
};

pub struct PosterSyncHandler {
    movie_repository: Arc<dyn MovieRepository>,
    metadata_client: Arc<dyn MetadataClient>,
    poster_fetcher: Arc<dyn PosterFetcherClient>,
    poster_storage: Arc<dyn PosterStorage>,
    max_retries: u32,
}

impl PosterSyncHandler {
    pub fn new(
        movie_repository: Arc<dyn MovieRepository>,
        metadata_client: Arc<dyn MetadataClient>,
        poster_fetcher: Arc<dyn PosterFetcherClient>,
        poster_storage: Arc<dyn PosterStorage>,
        max_retries: u32,
    ) -> Self {
        Self { movie_repository, metadata_client, poster_fetcher, poster_storage, max_retries }
    }

    async fn sync(&self, movie_id: MovieId, external_metadata_id: ExternalMetadataId) -> Result<(), DomainError> {
        let mut movie = match self.movie_repository.get_movie_by_id(&movie_id).await? {
            Some(m) => m,
            None => {
                tracing::warn!("Sync cancelled: Movie {} not found", movie_id.value());
                return Err(DomainError::NotFound("Movie not found".into()));
            }
        };

        let poster_url = match self.metadata_client.get_poster_url(&external_metadata_id).await {
            Ok(Some(url)) => url,
            Ok(None) => return Ok(()),
            Err(e) => {
                tracing::warn!("Failed to find poster URL: {:?}", e);
                return Err(e);
            }
        };

        let image_bytes = self.poster_fetcher.fetch_poster_bytes(&poster_url).await?;
        let stored_path = self.poster_storage.store_poster(&movie_id, &image_bytes).await?;

        movie.update_poster(stored_path);
        self.movie_repository.upsert_movie(&movie).await
    }
}

#[async_trait]
impl EventHandler for PosterSyncHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (movie_id, external_metadata_id) = match event {
            DomainEvent::MovieDiscovered { movie_id, external_metadata_id } => {
                (movie_id.value(), external_metadata_id.value().to_owned())
            }
            _ => return Ok(()),
        };

        let movie_id = MovieId::from_uuid(movie_id);
        let external_metadata_id = ExternalMetadataId::new(external_metadata_id)?;

        let mut last_err: Option<DomainError> = None;
        for attempt in 0..=self.max_retries {
            match self.sync(movie_id.clone(), external_metadata_id.clone()).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < self.max_retries {
                        let delay = Duration::from_secs(2u64.pow(attempt));
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = self.max_retries + 1,
                            delay_secs = delay.as_secs(),
                            "poster sync failed, retrying: {e}"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    last_err = Some(e);
                }
            }
        }

        let err = last_err.expect("loop runs at least once");
        tracing::error!(attempts = self.max_retries + 1, "poster sync failed after all attempts: {err}");
        Err(err)
    }
}
