use std::sync::Arc;

use domain::{
    ports::{
        AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher, ImageStorage,
        ImportProfileRepository, ImportSessionRepository, MetadataClient, MovieProfileRepository,
        MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
        ReviewRepository, SearchCommand, SearchPort, StatsRepository, UserProfileFieldsRepository,
        UserRepository, WatchlistRepository,
    },
    testing::{
        FakeAuthService, FakeDiaryRepository, FakeMetadataClient, FakePasswordHasher,
        InMemoryMovieRepository, InMemoryReviewRepository, InMemoryUserRepository,
        InMemoryWatchlistRepository, NoopEventPublisher, NoopImageStorage, PanicDiaryExporter,
        PanicDiaryRepository, PanicDocumentParser, PanicImportProfileRepository,
        PanicImportSessionRepository, PanicMovieProfileRepository, PanicPersonCommand,
        PanicPersonQuery, PanicPosterFetcher, PanicProfileFieldsRepo, PanicSearchCommand,
        PanicSearchPort, PanicStatsRepository,
    },
};
#[cfg(feature = "federation")]
use domain::testing::PanicRemoteWatchlistRepository;

use crate::{
    config::AppConfig,
    context::AppContext,
};

pub struct TestContextBuilder {
    pub movie_repo: Arc<dyn MovieRepository>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub diary_repo: Arc<dyn DiaryRepository>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub stats_repo: Arc<dyn StatsRepository>,
    pub metadata_client: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub image_storage: Arc<dyn ImageStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub user_repo: Arc<dyn UserRepository>,
    pub import_session_repo: Arc<dyn ImportSessionRepository>,
    pub import_profile_repo: Arc<dyn ImportProfileRepository>,
    pub movie_profile_repo: Arc<dyn MovieProfileRepository>,
    pub watchlist_repo: Arc<dyn WatchlistRepository>,
    pub profile_fields_repo: Arc<dyn UserProfileFieldsRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub config: AppConfig,
}

impl TestContextBuilder {
    pub fn new() -> Self {
        Self {
            movie_repo: InMemoryMovieRepository::new(),
            review_repo: InMemoryReviewRepository::new(),
            diary_repo: Arc::new(PanicDiaryRepository),
            diary_exporter: Arc::new(PanicDiaryExporter),
            document_parser: Arc::new(PanicDocumentParser),
            stats_repo: Arc::new(PanicStatsRepository),
            metadata_client: Arc::new(FakeMetadataClient),
            poster_fetcher: Arc::new(PanicPosterFetcher),
            image_storage: Arc::new(NoopImageStorage),
            event_publisher: NoopEventPublisher::new(),
            auth_service: Arc::new(FakeAuthService),
            password_hasher: Arc::new(FakePasswordHasher),
            user_repo: InMemoryUserRepository::new(),
            import_session_repo: Arc::new(PanicImportSessionRepository),
            import_profile_repo: Arc::new(PanicImportProfileRepository),
            movie_profile_repo: Arc::new(PanicMovieProfileRepository),
            watchlist_repo: InMemoryWatchlistRepository::new(),
            profile_fields_repo: Arc::new(PanicProfileFieldsRepo),
            person_command: Arc::new(PanicPersonCommand),
            person_query: Arc::new(PanicPersonQuery),
            search_port: Arc::new(PanicSearchPort),
            search_command: Arc::new(PanicSearchCommand),
            config: AppConfig {
                allow_registration: true,
                base_url: "http://localhost:3000".into(),
                rate_limit: 20,
            },
        }
    }

    pub fn with_movies(mut self, r: Arc<dyn MovieRepository>) -> Self {
        self.movie_repo = r;
        self
    }

    pub fn with_reviews(mut self, r: Arc<dyn ReviewRepository>) -> Self {
        self.review_repo = r;
        self
    }

    pub fn with_users(mut self, r: Arc<dyn UserRepository>) -> Self {
        self.user_repo = r;
        self
    }

    pub fn with_watchlist(mut self, r: Arc<dyn WatchlistRepository>) -> Self {
        self.watchlist_repo = r;
        self
    }

    pub fn with_diary(mut self, r: Arc<dyn DiaryRepository>) -> Self {
        self.diary_repo = r;
        self
    }

    pub fn with_event_publisher(mut self, p: Arc<dyn EventPublisher>) -> Self {
        self.event_publisher = p;
        self
    }

    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> AppContext {
        AppContext {
            movie_repository: self.movie_repo,
            review_repository: self.review_repo,
            diary_repository: self.diary_repo,
            diary_exporter: self.diary_exporter,
            document_parser: self.document_parser,
            stats_repository: self.stats_repo,
            metadata_client: self.metadata_client,
            poster_fetcher: self.poster_fetcher,
            image_storage: self.image_storage,
            event_publisher: self.event_publisher,
            auth_service: self.auth_service,
            password_hasher: self.password_hasher,
            user_repository: self.user_repo,
            import_session_repository: self.import_session_repo,
            import_profile_repository: self.import_profile_repo,
            movie_profile_repository: self.movie_profile_repo,
            watchlist_repository: self.watchlist_repo,
            profile_fields_repository: self.profile_fields_repo,
            person_command: self.person_command,
            person_query: self.person_query,
            search_port: self.search_port,
            search_command: self.search_command,
            config: self.config,
            #[cfg(feature = "federation")]
            remote_watchlist_repository: std::sync::Arc::new(PanicRemoteWatchlistRepository),
        }
    }
}
