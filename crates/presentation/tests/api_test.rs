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
        AuthService, EventPublisher, GeneratedToken, MetadataClient, MetadataSearchCriteria,
        PasswordHasher, PosterFetcherClient, PosterStorage, UserRepository,
    },
    value_objects::{
        Email, ExternalMetadataId, MovieId, PasswordHash, PosterPath, PosterUrl, UserId,
    },
};
use http_body_util::BodyExt;
use presentation::{routes, state::AppState};
use sqlite::SqliteMovieRepository;
use sqlx::SqlitePool;
use rss::RssAdapter;
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
    async fn get_poster_url(&self, _: &ExternalMetadataId) -> Result<Option<PosterUrl>, DomainError> {
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

struct PanicStorage;
#[async_trait]
impl PosterStorage for PanicStorage {
    async fn store_poster(&self, _: &MovieId, _: &[u8]) -> Result<PosterPath, DomainError> {
        panic!()
    }
    async fn get_poster(&self, _: &PosterPath) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
}

struct PanicHasher;
#[async_trait]
impl PasswordHasher for PanicHasher {
    async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> { panic!() }
    async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> { panic!() }
}

struct PanicAuth;
#[async_trait]
impl AuthService for PanicAuth {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> { panic!() }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> { panic!() }
}

struct NobodyUserRepo;
#[async_trait]
impl UserRepository for NobodyUserRepo {
    async fn find_by_email(&self, _: &Email) -> Result<Option<User>, DomainError> { Ok(None) }
    async fn find_by_username(&self, _: &domain::value_objects::Username) -> Result<Option<User>, DomainError> { Ok(None) }
    async fn save(&self, _: &User) -> Result<(), DomainError> { panic!() }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<User>, DomainError> { panic!() }
    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> { panic!() }
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
            stats_repository: Arc::clone(&repo) as _,
            metadata_client: Arc::new(PanicMeta),
            poster_fetcher: Arc::new(PanicFetcher),
            poster_storage: Arc::new(PanicStorage),
            event_publisher: Arc::new(NoopEventPublisher),
            auth_service: Arc::new(PanicAuth),
            password_hasher: Arc::new(PanicHasher),
            user_repository: Arc::new(NobodyUserRepo),
            config: AppConfig { allow_registration: false, base_url: "http://localhost:3000".to_string(), rate_limit: 20 },
        },
        html_renderer: Arc::new(AskamaHtmlRenderer::new()),
        rss_renderer: Arc::new(RssAdapter::new("http://localhost:3000".into())),
        ap_service: Arc::new(activitypub::NoopActivityPubService),
    };

    routes::build_router(state, axum::Router::new())
}

#[tokio::test]
async fn get_api_diary_returns_empty_list() {
    let app = test_app().await;
    let response = app
        .oneshot(Request::builder().uri("/api/diary").body(Body::empty()).unwrap())
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
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"rating":4,"watched_at":"2026-01-01T20:00:00","manual_title":"Dune","manual_release_year":2021}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_api_auth_login_unknown_user_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"a@b.com","password":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
