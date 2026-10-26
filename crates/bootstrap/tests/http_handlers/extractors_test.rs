use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use domain::{
    errors::DomainError, models::GeneratedToken, ports::AuthService, testing::InMemoryStore,
    value_objects::UserId,
};
use http_axum::extractors::{
    AdminApiUser, AdminUser, AuthenticatedUser, OptionalCookieUser, RequiredCookieUser,
};
use tower::ServiceExt;

use crate::support::{
    AcceptingAuth, ErroringUserRepo, make_test_state, test_app_state, test_app_state_with_user,
};

struct RejectingAuth;
#[async_trait::async_trait]
impl AuthService for RejectingAuth {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!()
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        Err(DomainError::Unauthorized("bad token".into()))
    }
}

// --- Routers ---

async fn protected_handler(user: AuthenticatedUser) -> String {
    user.0.value().to_string()
}
async fn optional_cookie_handler(user: OptionalCookieUser) -> String {
    match user.0 {
        Some(id) => id.value().to_string(),
        None => "none".to_string(),
    }
}
async fn required_cookie_handler(user: RequiredCookieUser) -> String {
    user.0.value().to_string()
}
async fn admin_api_handler(user: AdminApiUser) -> String {
    user.0.value().to_string()
}
async fn admin_cookie_handler(user: AdminUser) -> String {
    user.0.value().to_string()
}

fn router_protected(state: http_axum::state::AppState) -> Router {
    Router::new()
        .route("/protected", get(protected_handler))
        .with_state(state)
}
fn router_optional(state: http_axum::state::AppState) -> Router {
    Router::new()
        .route("/optional", get(optional_cookie_handler))
        .with_state(state)
}
fn router_admin_api(state: http_axum::state::AppState) -> Router {
    Router::new()
        .route("/admin-api", get(admin_api_handler))
        .with_state(state)
}
fn router_admin_cookie(state: http_axum::state::AppState) -> Router {
    Router::new()
        .route("/admin-cookie", get(admin_cookie_handler))
        .with_state(state)
}
fn router_required(state: http_axum::state::AppState) -> Router {
    Router::new()
        .route("/required", get(required_cookie_handler))
        .with_state(state)
}

// --- Tests ---

#[tokio::test]
async fn missing_auth_header_returns_401() {
    let app = router_protected(test_app_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn optional_cookie_user_returns_none_without_cookie() {
    let app = router_optional(test_app_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/optional")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn optional_cookie_user_returns_none_with_invalid_token() {
    let app = router_optional(make_test_state(Arc::new(RejectingAuth)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/optional")
                .header("cookie", "token=bad.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn required_cookie_user_redirects_without_cookie() {
    let app = router_required(test_app_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/required")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn required_cookie_user_redirects_with_invalid_token() {
    let app = router_required(make_test_state(Arc::new(RejectingAuth)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/required")
                .header("cookie", "token=bad.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

// --- AdminApiUser / AdminUser: per-path rejection pinning ---
//
// These pin the exact rejection for every path `authorize_admin::execute` can
// produce, now that both extractors go through it instead of calling
// `repos.user.find_by_id` directly. Each test name states the path; the
// asserted status is the byte-identical-to-before-this-task value recorded in
// the task-4 report's before/after table.

fn admin_id() -> UserId {
    UserId::generate()
}

#[tokio::test]
async fn admin_api_user_allows_an_admin_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new();
    user_repo.seed_user(domain::testing::test_user_with_role(
        id.clone(),
        domain::models::UserRole::Admin,
    ));
    let app = router_admin_api(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-api")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_api_user_rejects_a_non_admin_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new();
    user_repo.seed_user(domain::testing::test_user_with_role(
        id.clone(),
        domain::models::UserRole::Standard,
    ));
    let app = router_admin_api(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-api")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_api_user_rejects_an_unknown_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new(); // empty — id is not in the store
    let app = router_admin_api(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-api")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_api_user_surfaces_a_repository_error_as_500() {
    let id = admin_id();
    let app = router_admin_api(test_app_state_with_user(
        Arc::new(ErroringUserRepo),
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-api")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn admin_user_allows_an_admin_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new();
    user_repo.seed_user(domain::testing::test_user_with_role(
        id.clone(),
        domain::models::UserRole::Admin,
    ));
    let app = router_admin_cookie(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-cookie")
                .header("cookie", "token=anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_user_rejects_a_non_admin_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new();
    user_repo.seed_user(domain::testing::test_user_with_role(
        id.clone(),
        domain::models::UserRole::Standard,
    ));
    let app = router_admin_cookie(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-cookie")
                .header("cookie", "token=anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_user_rejects_an_unknown_user() {
    let id = admin_id();
    let user_repo = InMemoryStore::new(); // empty — id is not in the store
    let app = router_admin_cookie(test_app_state_with_user(
        user_repo as _,
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-cookie")
                .header("cookie", "token=anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn admin_user_surfaces_a_repository_error_as_500() {
    let id = admin_id();
    let app = router_admin_cookie(test_app_state_with_user(
        Arc::new(ErroringUserRepo),
        Arc::new(AcceptingAuth(id)),
    ));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin-cookie")
                .header("cookie", "token=anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn path_id_handler(
    http_axum::extractors::PathId(id): http_axum::extractors::PathId<u32>,
) -> String {
    id.to_string()
}

async fn refusal_code(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("refusal body is JSON");
    v["code"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn path_extractor_on_a_route_without_params_is_refused_as_internal() {
    let app = Router::new().route("/no-params", get(path_id_handler));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/no-params")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(refusal_code(resp).await, "INTERNAL");
}

#[tokio::test]
async fn path_extractor_with_an_unparseable_param_is_refused_as_invalid_input() {
    let app = Router::new().route("/items/{id}", get(path_id_handler));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/items/abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(refusal_code(resp).await, "INVALID_INPUT");
}
