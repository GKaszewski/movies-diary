use super::extractors::{make_test_state, Panic};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use domain::errors::DomainError;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

// Custom stub for SearchPort that returns empty results instead of panicking
struct SearchPortStub;
#[async_trait::async_trait]
impl domain::ports::SearchPort for SearchPortStub {
    async fn search(&self, _: &domain::models::SearchQuery) -> Result<domain::models::SearchResults, DomainError> {
        Ok(domain::models::SearchResults {
            movies: domain::models::collections::Paginated {
                items: vec![],
                total_count: 0,
                limit: 10,
                offset: 0,
            },
            people: domain::models::collections::Paginated {
                items: vec![],
                total_count: 0,
                limit: 10,
                offset: 0,
            },
        })
    }
}

// Custom stub for PersonQuery that returns 404 instead of panicking
struct PersonQueryStub;
#[async_trait::async_trait]
impl domain::ports::PersonQuery for PersonQueryStub {
    async fn get_by_id(&self, _: &domain::models::PersonId) -> Result<Option<domain::models::Person>, DomainError> {
        Ok(None)  // Return None to trigger 404
    }
    async fn get_by_external_id(&self, _: &domain::models::ExternalPersonId) -> Result<Option<domain::models::Person>, DomainError> {
        Ok(None)
    }
    async fn get_credits(&self, _: &domain::models::PersonId) -> Result<domain::models::PersonCredits, DomainError> {
        Err(DomainError::NotFound("Person not found".into()))
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<domain::models::PersonId>, DomainError> {
        Ok(vec![])
    }
}

// --- Search endpoint tests ---

#[tokio::test]
async fn search_endpoint_returns_200_with_empty_results() {
    let mut state = make_test_state(Arc::new(Panic));
    // Override the search_port with our stub
    state.app_ctx.search_port = Arc::new(SearchPortStub);
    let app = Router::new()
        .route("/api/v1/search", get(crate::handlers::api::get_search))
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=test&limit=10&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_endpoint_with_no_query_returns_200() {
    let mut state = make_test_state(Arc::new(Panic));
    // Override the search_port with our stub
    state.app_ctx.search_port = Arc::new(SearchPortStub);
    let app = Router::new()
        .route("/api/v1/search", get(crate::handlers::api::get_search))
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=&limit=5&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// --- Person endpoint tests ---

#[tokio::test]
async fn person_endpoint_returns_404_for_unknown_id() {
    let mut state = make_test_state(Arc::new(Panic));
    // Override the person_query with our stub
    state.app_ctx.person_query = Arc::new(PersonQueryStub);
    let app = Router::new()
        .route("/api/v1/people/{id}", get(crate::handlers::api::get_person_handler))
        .with_state(state);

    let unknown_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/people/{}", unknown_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn person_credits_endpoint_returns_404_for_unknown_id() {
    let mut state = make_test_state(Arc::new(Panic));
    // Override the person_query with our stub
    state.app_ctx.person_query = Arc::new(PersonQueryStub);
    let app = Router::new()
        .route("/api/v1/people/{id}/credits", get(crate::handlers::api::get_person_credits_handler))
        .with_state(state);

    let unknown_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/people/{}/credits", unknown_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- Watchlist endpoint tests ---

#[tokio::test]
async fn get_watchlist_requires_auth() {
    let state = make_test_state(Arc::new(Panic));
    let app = Router::new()
        .route("/api/v1/watchlist", get(crate::handlers::api::get_watchlist_handler))
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watchlist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_watchlist_status_requires_auth() {
    let state = make_test_state(Arc::new(Panic));
    let app = Router::new()
        .route(
            "/api/v1/watchlist/{movie_id}",
            get(crate::handlers::api::get_watchlist_status),
        )
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/watchlist/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
