use std::sync::Arc;

use application::config::AppConfig;
use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        EntityType, ExternalPersonId, GeneratedToken, IndexableDocument, MetadataSearchCriteria,
        Movie, Person, PersonCredits, PersonEnrichmentData, PersonId, SearchQuery, SearchResults,
        User,
    },
    ports::{
        AuthService, EventPublisher, MetadataClient, ObjectStorage, PasswordHasher, PersonCommand,
        PersonQuery, PosterFetcherClient, SearchCommand, SearchPort, UserRepository,
    },
    value_objects::{Email, ExternalMetadataId, PasswordHash, PosterUrl, UserId},
};
use http_body_util::BodyExt;
use presentation::context::{AppContext, Repositories, Services};
use presentation::{routes, state::AppState};
use rss::RssAdapter;
use sqlite::{
    SqliteDiaryRepository, SqliteMovieRepository, SqliteReviewRepository, SqliteStatsRepository,
    migrate as sqlite_migrate,
};
use sqlx::SqlitePool;
use tower::ServiceExt;

struct NoopEventPublisher;
#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
        Ok(())
    }
}

struct PanicReviewLogger;
#[async_trait]
impl application::ports::ReviewLogger for PanicReviewLogger {
    async fn log_review(
        &self,
        _: application::diary::commands::LogReviewCommand,
    ) -> Result<(), DomainError> {
        panic!("review_logger not wired in tests")
    }
}

struct PanicMeta;
#[async_trait]
impl MetadataClient for PanicMeta {
    async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> {
        panic!("metadata not wired in tests")
    }
    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        panic!()
    }
}

struct PanicFetcher;
#[async_trait]
impl PosterFetcherClient for PanicFetcher {
    async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
}

struct PanicObjectStorage;
#[async_trait]
impl ObjectStorage for PanicObjectStorage {
    async fn store(&self, _: &str, _: &[u8]) -> Result<String, DomainError> {
        panic!()
    }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
    async fn get_stream(
        &self,
        _: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        panic!()
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        panic!()
    }
}

struct PanicHasher;
#[async_trait]
impl PasswordHasher for PanicHasher {
    async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> {
        panic!()
    }
    async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> {
        panic!()
    }
}

struct PanicAuth;
#[async_trait]
impl AuthService for PanicAuth {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!()
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        panic!()
    }
}

struct NobodyUserRepo;
#[async_trait]
impl UserRepository for NobodyUserRepo {
    async fn find_by_email(&self, _: &Email) -> Result<Option<User>, DomainError> {
        Ok(None)
    }
    async fn find_by_username(
        &self,
        _: &domain::value_objects::Username,
    ) -> Result<Option<User>, DomainError> {
        Ok(None)
    }
    async fn save(&self, _: &User) -> Result<(), DomainError> {
        panic!()
    }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<User>, DomainError> {
        panic!()
    }
    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        panic!()
    }
    async fn update_profile(
        &self,
        _: &UserId,
        _: &domain::models::UserProfile,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

struct PanicProfileFields;
#[async_trait]
impl domain::ports::UserProfileFieldsRepository for PanicProfileFields {
    async fn get_fields(
        &self,
        _: &UserId,
    ) -> Result<Vec<domain::models::ProfileField>, DomainError> {
        Ok(vec![])
    }
    async fn set_fields(
        &self,
        _: &UserId,
        _: Vec<domain::models::ProfileField>,
    ) -> Result<(), DomainError> {
        panic!()
    }
}

struct PanicExporter;
impl domain::ports::DiaryExporter for PanicExporter {
    fn stream_entries(
        &self,
        _stream: futures::stream::BoxStream<
            'static,
            Result<domain::models::DiaryEntry, DomainError>,
        >,
        _format: domain::models::ExportFormat,
    ) -> futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>> {
        panic!("PanicExporter::stream_entries")
    }
}

struct PanicImportSession;
#[async_trait]
impl domain::ports::ImportSessionRepository for PanicImportSession {
    async fn create(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> {
        panic!()
    }
    async fn get(
        &self,
        _: &domain::value_objects::ImportSessionId,
        _: &UserId,
    ) -> Result<Option<domain::models::ImportSession>, DomainError> {
        panic!()
    }
    async fn update(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete(&self, _: &domain::value_objects::ImportSessionId) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete_expired(&self) -> Result<u64, DomainError> {
        panic!()
    }
    async fn delete_expired_for_user(&self, _: &UserId) -> Result<(), DomainError> {
        panic!()
    }
}

struct PanicDocumentParser;
impl domain::ports::DocumentParser for PanicDocumentParser {
    fn parse(
        &self,
        _: &[u8],
        _: domain::models::FileFormat,
    ) -> Result<domain::models::ParsedFile, domain::models::ImportError> {
        panic!("DocumentParser not wired in tests")
    }
    fn apply_mapping(
        &self,
        _: &domain::models::ParsedFile,
        _: &[domain::models::FieldMapping],
    ) -> Vec<domain::models::AnnotatedRow> {
        panic!("DocumentParser not wired in tests")
    }
}

struct PanicImportProfile;

struct PanicMovieProfile;
#[async_trait]
impl domain::ports::MovieProfileRepository for PanicMovieProfile {
    async fn upsert(&self, _: &domain::models::MovieProfile) -> Result<(), DomainError> {
        panic!()
    }
    async fn get_by_movie_id(
        &self,
        _: &domain::value_objects::MovieId,
    ) -> Result<Option<domain::models::MovieProfile>, DomainError> {
        Ok(None)
    }
    async fn list_stale(
        &self,
    ) -> Result<Vec<(domain::value_objects::MovieId, String)>, DomainError> {
        Ok(vec![])
    }
}
#[async_trait]
impl domain::ports::ImportProfileRepository for PanicImportProfile {
    async fn save(&self, _: &domain::models::ImportProfile) -> Result<(), DomainError> {
        panic!()
    }
    async fn list_for_user(
        &self,
        _: &UserId,
    ) -> Result<Vec<domain::models::ImportProfile>, DomainError> {
        panic!()
    }
    async fn get(
        &self,
        _: &domain::value_objects::ImportProfileId,
        _: &UserId,
    ) -> Result<Option<domain::models::ImportProfile>, DomainError> {
        panic!()
    }
    async fn delete(&self, _: &domain::value_objects::ImportProfileId) -> Result<(), DomainError> {
        panic!()
    }
}

struct PanicWatchlist;
#[async_trait]
impl domain::ports::WatchlistRepository for PanicWatchlist {
    async fn add(&self, _: &domain::models::WatchlistEntry) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove_if_present(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_for_user(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::models::collections::PageParams,
    ) -> Result<
        domain::models::collections::Paginated<domain::models::WatchlistWithMovie>,
        DomainError,
    > {
        panic!()
    }
    async fn contains(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}

struct PanicPersonCommand;
#[async_trait]
impl PersonCommand for PanicPersonCommand {
    async fn upsert_batch(&self, _: &[Person]) -> Result<(), DomainError> {
        panic!()
    }
    async fn backfill_from_credits_batch(
        &self,
        _batch_size: u32,
    ) -> Result<(u64, bool), DomainError> {
        panic!()
    }
    async fn update_enrichment(
        &self,
        _: &PersonId,
        _: &PersonEnrichmentData,
    ) -> Result<(), DomainError> {
        panic!()
    }
}

struct PanicPersonQuery;
#[async_trait]
impl PersonQuery for PanicPersonQuery {
    async fn get_by_id(&self, _: &PersonId) -> Result<Option<Person>, DomainError> {
        panic!()
    }
    async fn get_by_external_id(
        &self,
        _: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        panic!()
    }
    async fn get_credits(&self, _: &PersonId) -> Result<PersonCredits, DomainError> {
        panic!()
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        panic!()
    }
    async fn list_page(
        &self,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<domain::models::Person>, DomainError> {
        panic!()
    }
}

struct PanicSearchPort;
#[async_trait]
impl SearchPort for PanicSearchPort {
    async fn search(&self, _: &SearchQuery) -> Result<SearchResults, DomainError> {
        panic!()
    }
}

struct PanicSearchCommand;
#[async_trait]
impl SearchCommand for PanicSearchCommand {
    async fn index(&self, _: IndexableDocument) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove(&self, _: EntityType, _: &str) -> Result<(), DomainError> {
        panic!()
    }
}

#[cfg(feature = "federation")]
struct PanicRemoteWatchlist;
#[cfg(feature = "federation")]
#[async_trait::async_trait]
impl domain::ports::RemoteWatchlistRepository for PanicRemoteWatchlist {
    async fn save(&self, _: domain::models::RemoteWatchlistEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<domain::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_derived_uuid(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<domain::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
}
async fn test_app() -> Router {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite failed");
    sqlite_migrate(&pool).await.expect("migration failed");

    let state = AppState {
        app_ctx: AppContext {
            repos: Repositories {
                movie_command: Arc::new(SqliteMovieRepository::new(pool.clone())) as _,
                movie_query: Arc::new(SqliteMovieRepository::new(pool.clone())) as _,
                review: Arc::new(SqliteReviewRepository::new(pool.clone())) as _,
                diary: Arc::new(SqliteDiaryRepository::new(pool.clone())) as _,
                stats: Arc::new(SqliteStatsRepository::new(pool.clone())) as _,
                user: Arc::new(NobodyUserRepo),
                import_session: Arc::new(PanicImportSession),
                import_profile: Arc::new(PanicImportProfile),
                movie_profile: Arc::new(PanicMovieProfile),
                watchlist: Arc::new(PanicWatchlist),
                watch_event_command: Arc::new(domain::testing::PanicWatchEventCommand),
                watch_event_query: Arc::new(domain::testing::PanicWatchEventQuery),
                webhook_token: Arc::new(domain::testing::PanicWebhookTokenRepository),
                profile_fields: Arc::new(PanicProfileFields),
                person_command: Arc::new(PanicPersonCommand),
                person_query: Arc::new(PanicPersonQuery),
                search_port: Arc::new(PanicSearchPort),
                search_command: Arc::new(PanicSearchCommand),
                remote_watchlist: Arc::new(PanicRemoteWatchlist),
                social_command: Arc::new(domain::ports::noop::NoopSocialCommand),
                social_query_unified: Arc::new(domain::ports::noop::NoopSocialQuery),
                federation_admin: Arc::new(domain::ports::noop::NoopFederationAdminQuery) as _,
                wrapup_stats: Arc::new(domain::testing::PanicWrapUpStatsQuery) as _,
                wrapup_repo: Arc::new(domain::testing::PanicWrapUpRepository) as _,
                goal_command: Arc::new(domain::testing::NoopGoalCommand),
                goal_query: Arc::new(domain::testing::NoopGoalQuery),
                user_settings: Arc::new(domain::testing::NoopUserSettingsRepository),
                remote_goal: Arc::new(domain::testing::NoopRemoteGoalRepository),
                refresh_session: Arc::new(domain::testing::PanicRefreshSessionRepository),
                federated_profile: None,
            },
            services: Services {
                auth: Arc::new(PanicAuth),
                password_hasher: Arc::new(PanicHasher),
                metadata: Arc::new(PanicMeta),
                poster_fetcher: Arc::new(PanicFetcher),
                object_storage: Arc::new(PanicObjectStorage),
                event_publisher: Arc::new(NoopEventPublisher),
                diary_exporter: Arc::new(PanicExporter),
                document_parser: Arc::new(PanicDocumentParser),
                review_logger: Arc::new(PanicReviewLogger),
                person_enrichment: None,
                #[cfg(feature = "federation")]
                ap_service: Arc::new(activitypub::NoopActivityPubService),
            },
            config: AppConfig {
                allow_registration: false,
                base_url: "http://localhost:3000".to_string(),
                rate_limit: 20,
                refresh_ttl_seconds: 2_592_000,
                wrapup: application::config::WrapUpConfig {
                    font_path: None,
                    logo_path: None,
                    bg_dir: None,
                },
            },
        },
        rss_renderer: Arc::new(RssAdapter::new("http://localhost:3000".into())),
    };

    routes::build_router(state, axum::Router::new())
}

/// Inject a fake peer IP so the GovernorLayer can extract ConnectInfo.
fn with_ip(req: Request<Body>) -> Request<Body> {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    let mut req = req;
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo::<std::net::SocketAddr>(addr));
    req
}

#[tokio::test]
async fn get_api_diary_returns_empty_list() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/api/v1/diary")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["total_count"], 0);
    assert_eq!(json["items"], serde_json::json!([]));
    assert_eq!(json["limit"], 5);
    assert_eq!(json["offset"], 0);
}

#[tokio::test]
async fn post_api_reviews_without_auth_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"rating":4,"watched_at":"2026-01-01T20:00:00","manual_title":"Dune","manual_release_year":2021}"#,
                ))
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_api_auth_login_unknown_user_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"a@b.com","password":"x"}"#))
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_api_movie_detail_returns_404_for_unknown_id() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/api/v1/movies/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tags_moviesdiary_redirects_to_home() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/tags/moviesdiary")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(response.headers().get("location").unwrap(), "/");
}

#[tokio::test]
async fn tags_other_redirects_to_search() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/tags/batman")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/?search=batman"
    );
}

#[tokio::test]
async fn get_movie_detail_html_returns_404_for_unknown_id() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/movies/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
