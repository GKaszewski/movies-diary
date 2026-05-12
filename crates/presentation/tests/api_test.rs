use std::sync::Arc;

use application::{config::AppConfig, context::AppContext};
use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, User},
    ports::{
        AuthService, EventPublisher, GeneratedToken, ImageStorage, MetadataClient, MetadataSearchCriteria,
        PasswordHasher, PosterFetcherClient, UserRepository,
    },
    value_objects::{
        Email, ExternalMetadataId, PasswordHash, PosterUrl, UserId,
    },
};
use http_body_util::BodyExt;
use presentation::{routes, state::AppState};
use rss::RssAdapter;
use sqlite::SqliteMovieRepository;
use sqlx::SqlitePool;
use template_askama::AskamaHtmlRenderer;
use tower::ServiceExt;

struct NoopEventPublisher;
#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
        Ok(())
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

struct PanicImageStorage;
#[async_trait]
impl ImageStorage for PanicImageStorage {
    async fn store(&self, _: &str, _: &[u8]) -> Result<String, DomainError> { panic!() }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> { panic!() }
    async fn delete(&self, _: &str) -> Result<(), DomainError> { panic!() }
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
    async fn update_profile(&self, _: &UserId, _: Option<String>, _: Option<String>) -> Result<(), DomainError> {
        Ok(())
    }
}

struct PanicExporter;
#[async_trait]
impl domain::ports::DiaryExporter for PanicExporter {
    async fn serialize_entries(
        &self,
        _: &[domain::models::DiaryEntry],
        _: domain::models::ExportFormat,
    ) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
}

struct PanicImportSession;
#[async_trait]
impl domain::ports::ImportSessionRepository for PanicImportSession {
    async fn create(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> { panic!() }
    async fn get(&self, _: &domain::value_objects::ImportSessionId, _: &UserId) -> Result<Option<domain::models::ImportSession>, DomainError> { panic!() }
    async fn update(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> { panic!() }
    async fn delete(&self, _: &domain::value_objects::ImportSessionId) -> Result<(), DomainError> { panic!() }
    async fn delete_expired(&self) -> Result<u64, DomainError> { panic!() }
    async fn delete_expired_for_user(&self, _: &UserId) -> Result<(), DomainError> { panic!() }
}

struct PanicDocumentParser;
impl domain::ports::DocumentParser for PanicDocumentParser {
    fn parse(&self, _: &[u8], _: domain::models::FileFormat) -> Result<domain::models::ParsedFile, domain::models::ImportError> {
        panic!("DocumentParser not wired in tests")
    }
    fn apply_mapping(&self, _: &domain::models::ParsedFile, _: &[domain::models::FieldMapping]) -> Vec<domain::models::AnnotatedRow> {
        panic!("DocumentParser not wired in tests")
    }
}

struct PanicImportProfile;

struct PanicMovieProfile;
#[async_trait]
impl domain::ports::MovieProfileRepository for PanicMovieProfile {
    async fn upsert(&self, _: &domain::models::MovieProfile) -> Result<(), DomainError> { panic!() }
    async fn get_by_movie_id(&self, _: &domain::value_objects::MovieId) -> Result<Option<domain::models::MovieProfile>, DomainError> { Ok(None) }
    async fn list_stale(&self) -> Result<Vec<(domain::value_objects::MovieId, String)>, DomainError> { Ok(vec![]) }
}
#[async_trait]
impl domain::ports::ImportProfileRepository for PanicImportProfile {
    async fn save(&self, _: &domain::models::ImportProfile) -> Result<(), DomainError> { panic!() }
    async fn list_for_user(&self, _: &UserId) -> Result<Vec<domain::models::ImportProfile>, DomainError> { panic!() }
    async fn get(&self, _: &domain::value_objects::ImportProfileId, _: &UserId) -> Result<Option<domain::models::ImportProfile>, DomainError> { panic!() }
    async fn delete(&self, _: &domain::value_objects::ImportProfileId) -> Result<(), DomainError> { panic!() }
}

#[cfg(feature = "federation")]
struct PanicSocialQuery;
#[cfg(feature = "federation")]
#[async_trait::async_trait]
impl domain::ports::SocialQueryPort for PanicSocialQuery {
    async fn get_accepted_following_urls(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<String>, DomainError> {
        panic!()
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<domain::ports::RemoteActorInfo>, DomainError> {
        panic!()
    }
}

async fn test_app() -> Router {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite failed");
    let repo = SqliteMovieRepository::new(pool);
    repo.migrate().await.expect("migration failed");

    let repo = Arc::new(repo);
    let state = AppState {
        app_ctx: AppContext {
            movie_repository: Arc::clone(&repo) as _,
            review_repository: Arc::clone(&repo) as _,
            diary_repository: Arc::clone(&repo) as _,
            diary_exporter: Arc::new(PanicExporter),
            document_parser: Arc::new(PanicDocumentParser),
            stats_repository: Arc::clone(&repo) as _,
            metadata_client: Arc::new(PanicMeta),
            poster_fetcher: Arc::new(PanicFetcher),
            image_storage: Arc::new(PanicImageStorage),
            event_publisher: Arc::new(NoopEventPublisher),
            auth_service: Arc::new(PanicAuth),
            password_hasher: Arc::new(PanicHasher),
            user_repository: Arc::new(NobodyUserRepo),
            import_session_repository: Arc::new(PanicImportSession),
            import_profile_repository: Arc::new(PanicImportProfile),
            movie_profile_repository: Arc::new(PanicMovieProfile),
            config: AppConfig {
                allow_registration: false,
                base_url: "http://localhost:3000".to_string(),
                rate_limit: 20,
            },
        },
        html_renderer: Arc::new(AskamaHtmlRenderer::new()),
        rss_renderer: Arc::new(RssAdapter::new("http://localhost:3000".into())),
        #[cfg(feature = "federation")]
        ap_service: Arc::new(activitypub::NoopActivityPubService),
        #[cfg(feature = "federation")]
        social_query: Arc::new(PanicSocialQuery),
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
    assert_eq!(response.headers().get("location").unwrap(), "/?search=batman");
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
