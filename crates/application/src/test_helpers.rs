use std::sync::Arc;

use domain::testing::{
    InMemoryWrapUpRepository, InMemoryWrapUpStatsQuery, NoopRemoteWatchlistRepository,
    NoopSocialQueryPort,
};
use domain::{
    ports::{
        AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher,
        ImportProfileRepository, ImportSessionRepository, MetadataClient, MovieProfileRepository,
        MovieRepository, ObjectStorage, PasswordHasher, PersonCommand, PersonQuery,
        PosterFetcherClient, ReviewRepository, SearchCommand, SearchPort, StatsRepository,
        UserProfileFieldsRepository, UserRepository, WatchEventRepository, WatchlistRepository,
        WebhookTokenRepository, WrapUpRepository, WrapUpStatsQuery,
    },
    testing::{
        FakeAuthService, FakeMetadataClient, FakePasswordHasher, InMemoryMovieRepository,
        InMemoryReviewRepository, InMemoryUserRepository, InMemoryWatchlistRepository,
        NoopEventPublisher, NoopObjectStorage, PanicDiaryExporter, PanicDiaryRepository,
        PanicDocumentParser, PanicImportProfileRepository, PanicImportSessionRepository,
        PanicMovieProfileRepository, PanicPersonCommand, PanicPersonQuery, PanicPosterFetcher,
        PanicProfileFieldsRepo, PanicSearchCommand, PanicSearchPort, PanicStatsRepository,
        PanicWatchEventRepository, PanicWebhookTokenRepository,
    },
};

use crate::{
    config::AppConfig,
    context::{AppContext, Repositories, Services},
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
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub user_repo: Arc<dyn UserRepository>,
    pub import_session_repo: Arc<dyn ImportSessionRepository>,
    pub import_profile_repo: Arc<dyn ImportProfileRepository>,
    pub movie_profile_repo: Arc<dyn MovieProfileRepository>,
    pub watchlist_repo: Arc<dyn WatchlistRepository>,
    pub watch_event_repo: Arc<dyn WatchEventRepository>,
    pub webhook_token_repo: Arc<dyn WebhookTokenRepository>,
    pub profile_fields_repo: Arc<dyn UserProfileFieldsRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub wrapup_stats: Arc<dyn WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
    pub config: AppConfig,
}

impl Default for TestContextBuilder {
    fn default() -> Self {
        Self::new()
    }
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
            object_storage: Arc::new(NoopObjectStorage),
            event_publisher: NoopEventPublisher::new(),
            auth_service: Arc::new(FakeAuthService),
            password_hasher: Arc::new(FakePasswordHasher),
            user_repo: InMemoryUserRepository::new(),
            import_session_repo: Arc::new(PanicImportSessionRepository),
            import_profile_repo: Arc::new(PanicImportProfileRepository),
            movie_profile_repo: Arc::new(PanicMovieProfileRepository),
            watchlist_repo: InMemoryWatchlistRepository::new(),
            watch_event_repo: Arc::new(PanicWatchEventRepository),
            webhook_token_repo: Arc::new(PanicWebhookTokenRepository),
            profile_fields_repo: Arc::new(PanicProfileFieldsRepo),
            person_command: Arc::new(PanicPersonCommand),
            person_query: Arc::new(PanicPersonQuery),
            search_port: Arc::new(PanicSearchPort),
            search_command: Arc::new(PanicSearchCommand),
            wrapup_stats: InMemoryWrapUpStatsQuery::new(),
            wrapup_repo: InMemoryWrapUpRepository::new(),
            config: AppConfig {
                allow_registration: true,
                base_url: "http://localhost:3000".into(),
                rate_limit: 20,
                wrapup: crate::config::WrapUpConfig {
                    font_path: None,
                    logo_path: None,
                    bg_dir: None,
                    ffmpeg_path: "ffmpeg".into(),
                    max_concurrent_renders: 2,
                },
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

    pub fn wrapup_stats(mut self, r: Arc<dyn WrapUpStatsQuery>) -> Self {
        self.wrapup_stats = r;
        self
    }

    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> AppContext {
        AppContext {
            repos: Repositories {
                movie: self.movie_repo,
                review: self.review_repo,
                diary: self.diary_repo,
                stats: self.stats_repo,
                user: self.user_repo,
                import_session: self.import_session_repo,
                import_profile: self.import_profile_repo,
                movie_profile: self.movie_profile_repo,
                watchlist: self.watchlist_repo,
                watch_event: self.watch_event_repo,
                webhook_token: self.webhook_token_repo,
                profile_fields: self.profile_fields_repo,
                person_command: self.person_command,
                person_query: self.person_query,
                search_port: self.search_port,
                search_command: self.search_command,
                remote_watchlist: Arc::new(NoopRemoteWatchlistRepository),
                social_query: Arc::new(NoopSocialQueryPort),
                wrapup_stats: self.wrapup_stats,
                wrapup_repo: self.wrapup_repo,
                goal: Arc::new(domain::testing::NoopGoalRepository),
                user_settings: Arc::new(domain::testing::NoopUserSettingsRepository),
                remote_goal: Arc::new(domain::testing::NoopRemoteGoalRepository),
            },
            services: Services {
                auth: self.auth_service,
                password_hasher: self.password_hasher,
                metadata: self.metadata_client,
                poster_fetcher: self.poster_fetcher,
                object_storage: self.object_storage,
                event_publisher: self.event_publisher,
                diary_exporter: self.diary_exporter,
                document_parser: self.document_parser,
                video_renderer: None,
            },
            config: self.config,
        }
    }
}
