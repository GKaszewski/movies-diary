use std::time::Duration;

use application::{commands::SyncPosterCommand, context::AppContext, use_cases::sync_poster};
use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent};
use event_publisher::EventHandler;

pub struct PosterSyncHandler {
    ctx: AppContext,
    max_retries: u32,
}

impl PosterSyncHandler {
    pub fn new(ctx: AppContext, max_retries: u32) -> Self {
        Self { ctx, max_retries }
    }
}

#[async_trait]
impl EventHandler for PosterSyncHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (movie_id, external_metadata_id) = match event {
            DomainEvent::MovieDiscovered {
                movie_id,
                external_metadata_id,
            } => (movie_id.value(), external_metadata_id.value().to_owned()),
            _ => return Ok(()),
        };

        let mut last_err: Option<DomainError> = None;
        for attempt in 0..=self.max_retries {
            let cmd = SyncPosterCommand {
                movie_id,
                external_metadata_id: external_metadata_id.clone(),
            };
            match sync_poster::execute(&self.ctx, cmd).await {
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

        let err = last_err.expect("loop runs at least once and always sets last_err on Err");
        tracing::error!(
            attempts = self.max_retries + 1,
            "poster sync failed after all attempts: {err}"
        );
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use application::config::AppConfig;
    use domain::{
        errors::DomainError,
        events::DomainEvent,
        models::{DiaryEntry, DiaryFilter, Movie, Review, ReviewHistory, User, collections::Paginated},
        ports::{
            AuthService, EventPublisher, GeneratedToken, MetadataClient, MetadataSearchCriteria,
            MovieRepository, PasswordHasher, PosterFetcherClient, PosterStorage, UserRepository,
        },
        value_objects::{
            Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, PosterUrl,
            Rating, ReleaseYear, ReviewId, UserId,
        },
    };

    // Panic-stub ports: each method panics so any accidental dispatch into a service
    // fails the test loudly rather than silently succeeding.
    struct PanicRepo;
    struct PanicMetadata;
    struct PanicFetcher;
    struct PanicStorage;
    struct PanicAuth;
    struct PanicHasher;
    struct PanicUserRepo;
    struct NoopPublisher;

    #[async_trait]
    impl MovieRepository for PanicRepo {
        async fn get_movie_by_external_id(&self, _: &ExternalMetadataId) -> Result<Option<Movie>, DomainError> { panic!("unexpected") }
        async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> { panic!("unexpected") }
        async fn get_movies_by_title_and_year(&self, _: &MovieTitle, _: &ReleaseYear) -> Result<Vec<Movie>, DomainError> { panic!("unexpected") }
        async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> { panic!("unexpected") }
        async fn save_review(&self, _: &Review) -> Result<DomainEvent, DomainError> { panic!("unexpected") }
        async fn query_diary(&self, _: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> { panic!("unexpected") }
        async fn get_review_history(&self, _: &MovieId) -> Result<ReviewHistory, DomainError> { panic!("unexpected") }
        async fn get_review_by_id(&self, _: &ReviewId) -> Result<Option<Review>, DomainError> { panic!("unexpected") }
        async fn delete_review(&self, _: &ReviewId) -> Result<(), DomainError> { panic!("unexpected") }
        async fn delete_movie(&self, _: &MovieId) -> Result<(), DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl MetadataClient for PanicMetadata {
        async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> { panic!("unexpected") }
        async fn get_poster_url(&self, _: &ExternalMetadataId) -> Result<Option<PosterUrl>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PosterFetcherClient for PanicFetcher {
        async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PosterStorage for PanicStorage {
        async fn store_poster(&self, _: &MovieId, _: &[u8]) -> Result<PosterPath, DomainError> { panic!("unexpected") }
        async fn get_poster(&self, _: &PosterPath) -> Result<Vec<u8>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl AuthService for PanicAuth {
        async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> { panic!("unexpected") }
        async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PasswordHasher for PanicHasher {
        async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> { panic!("unexpected") }
        async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl UserRepository for PanicUserRepo {
        async fn find_by_email(&self, _: &Email) -> Result<Option<User>, DomainError> { panic!("unexpected") }
        async fn save(&self, _: &User) -> Result<(), DomainError> { panic!("unexpected") }
        async fn find_by_id(&self, _: &domain::value_objects::UserId) -> Result<Option<User>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl EventPublisher for NoopPublisher {
        async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> { Ok(()) }
    }

    fn panic_ctx() -> AppContext {
        AppContext {
            repository: Arc::new(PanicRepo),
            metadata_client: Arc::new(PanicMetadata),
            poster_fetcher: Arc::new(PanicFetcher),
            poster_storage: Arc::new(PanicStorage),
            event_publisher: Arc::new(NoopPublisher),
            auth_service: Arc::new(PanicAuth),
            password_hasher: Arc::new(PanicHasher),
            user_repository: Arc::new(PanicUserRepo),
            config: AppConfig { allow_registration: false },
        }
    }

    #[tokio::test]
    async fn review_logged_is_ignored() {
        let handler = PosterSyncHandler::new(panic_ctx(), 3);
        let event = DomainEvent::ReviewLogged {
            review_id: ReviewId::generate(),
            movie_id: MovieId::generate(),
            user_id: UserId::generate(),
            rating: Rating::new(4).unwrap(),
            watched_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        };
        assert!(handler.handle(&event).await.is_ok());
    }
}
