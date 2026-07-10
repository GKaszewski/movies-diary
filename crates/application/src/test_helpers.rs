use std::sync::Arc;

use domain::testing::{
    InMemoryGoalRepository, InMemoryWrapUpRepository, InMemoryWrapUpStatsQuery, NoopSocialQueryPort,
};
use domain::{
    ports::{
        AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher,
        GoalRepository, ImportProfileRepository, ImportSessionRepository, MetadataClient,
        MovieProfileRepository, MovieRepository, ObjectStorage, PasswordHasher, PersonCommand,
        PersonQuery, PosterFetcherClient, RefreshSessionRepository, ReviewRepository,
        SearchCommand, SearchPort, StatsRepository, UserProfileFieldsRepository, UserRepository,
        UserSettingsRepository, WatchEventRepository, WatchlistRepository, WebhookTokenRepository,
        WrapUpRepository, WrapUpStatsQuery,
    },
    testing::{
        FakeAuthService, FakeDiaryRepository, FakeDocumentParser, FakeMetadataClient,
        FakePasswordHasher, FakePersonQuery, FakePosterFetcher, FakeSearchCommand, FakeSearchPort,
        FakeStatsRepository, InMemoryImportProfileRepository, InMemoryImportSessionRepository,
        InMemoryMovieProfileRepository, InMemoryMovieRepository, InMemoryProfileFieldsRepo,
        InMemoryRefreshSessionRepository, InMemoryReviewRepository, InMemoryUserRepository,
        InMemoryUserSettingsRepository, InMemoryWatchEventRepository, InMemoryWatchlistRepository,
        InMemoryWebhookTokenRepository, NoopEventPublisher, NoopObjectStorage, PanicDiaryExporter,
        PanicPersonCommand,
    },
};

use async_trait::async_trait;
use domain::errors::DomainError;

use crate::{config::AppConfig, diary::commands::LogReviewCommand, ports::ReviewLogger};

pub struct NoopReviewLogger;

#[async_trait]
impl ReviewLogger for NoopReviewLogger {
    async fn log_review(&self, _cmd: LogReviewCommand) -> Result<(), DomainError> {
        Ok(())
    }
}

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
    pub goal_repo: Arc<dyn GoalRepository>,
    pub user_settings_repo: Arc<dyn UserSettingsRepository>,
    pub review_logger: Arc<dyn ReviewLogger>,
    pub social_query: Arc<dyn domain::ports::SocialQueryPort>,
    pub refresh_session_repo: Arc<dyn RefreshSessionRepository>,
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
            diary_repo: FakeDiaryRepository::new(),
            diary_exporter: Arc::new(PanicDiaryExporter),
            document_parser: Arc::new(FakeDocumentParser),
            stats_repo: FakeStatsRepository::new(),
            metadata_client: Arc::new(FakeMetadataClient),
            poster_fetcher: Arc::new(FakePosterFetcher),
            object_storage: Arc::new(NoopObjectStorage),
            event_publisher: NoopEventPublisher::new(),
            auth_service: Arc::new(FakeAuthService),
            password_hasher: Arc::new(FakePasswordHasher),
            user_repo: InMemoryUserRepository::new(),
            import_session_repo: InMemoryImportSessionRepository::new(),
            import_profile_repo: InMemoryImportProfileRepository::new(),
            movie_profile_repo: InMemoryMovieProfileRepository::new(),
            watchlist_repo: InMemoryWatchlistRepository::new(),
            watch_event_repo: InMemoryWatchEventRepository::new(),
            webhook_token_repo: InMemoryWebhookTokenRepository::new(),
            profile_fields_repo: InMemoryProfileFieldsRepo::new(),
            person_command: Arc::new(PanicPersonCommand),
            person_query: Arc::new(FakePersonQuery),
            search_port: Arc::new(FakeSearchPort),
            search_command: Arc::new(FakeSearchCommand),
            wrapup_stats: InMemoryWrapUpStatsQuery::new(),
            wrapup_repo: InMemoryWrapUpRepository::new(),
            goal_repo: InMemoryGoalRepository::new(),
            user_settings_repo: InMemoryUserSettingsRepository::new(),
            review_logger: Arc::new(NoopReviewLogger),
            social_query: Arc::new(NoopSocialQueryPort),
            refresh_session_repo: InMemoryRefreshSessionRepository::new(),
            config: AppConfig {
                allow_registration: true,
                base_url: "http://localhost:3000".into(),
                rate_limit: 20,
                refresh_ttl_seconds: 2_592_000,
                wrapup: crate::config::WrapUpConfig {
                    font_path: None,
                    logo_path: None,
                    bg_dir: None,
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

    pub fn with_goal(mut self, r: Arc<dyn GoalRepository>) -> Self {
        self.goal_repo = r;
        self
    }

    pub fn with_webhook_tokens(mut self, r: Arc<dyn WebhookTokenRepository>) -> Self {
        self.webhook_token_repo = r;
        self
    }

    pub fn with_watch_events(mut self, r: Arc<dyn WatchEventRepository>) -> Self {
        self.watch_event_repo = r;
        self
    }

    pub fn with_import_sessions(mut self, r: Arc<dyn ImportSessionRepository>) -> Self {
        self.import_session_repo = r;
        self
    }

    pub fn with_import_profiles(mut self, r: Arc<dyn ImportProfileRepository>) -> Self {
        self.import_profile_repo = r;
        self
    }

    pub fn with_movie_profiles(mut self, r: Arc<dyn MovieProfileRepository>) -> Self {
        self.movie_profile_repo = r;
        self
    }

    pub fn with_user_settings(mut self, r: Arc<dyn UserSettingsRepository>) -> Self {
        self.user_settings_repo = r;
        self
    }

    pub fn with_profile_fields(mut self, r: Arc<dyn UserProfileFieldsRepository>) -> Self {
        self.profile_fields_repo = r;
        self
    }

    pub fn with_review_logger(mut self, r: Arc<dyn ReviewLogger>) -> Self {
        self.review_logger = r;
        self
    }

    pub fn with_stats(mut self, r: Arc<dyn StatsRepository>) -> Self {
        self.stats_repo = r;
        self
    }

    pub fn with_person_query(mut self, r: Arc<dyn PersonQuery>) -> Self {
        self.person_query = r;
        self
    }

    pub fn with_search_port(mut self, r: Arc<dyn SearchPort>) -> Self {
        self.search_port = r;
        self
    }

    pub fn with_search_command(mut self, r: Arc<dyn SearchCommand>) -> Self {
        self.search_command = r;
        self
    }

    pub fn with_document_parser(mut self, r: Arc<dyn DocumentParser>) -> Self {
        self.document_parser = r;
        self
    }

    pub fn with_poster_fetcher(mut self, r: Arc<dyn PosterFetcherClient>) -> Self {
        self.poster_fetcher = r;
        self
    }

    pub fn with_metadata_client(mut self, r: Arc<dyn MetadataClient>) -> Self {
        self.metadata_client = r;
        self
    }

    pub fn with_social_query(mut self, r: Arc<dyn domain::ports::SocialQueryPort>) -> Self {
        self.social_query = r;
        self
    }

    pub fn with_wrapup_repo(mut self, r: Arc<dyn WrapUpRepository>) -> Self {
        self.wrapup_repo = r;
        self
    }

    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }
}
