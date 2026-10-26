use crate::support::{AcceptingAuth, test_app_state, test_app_state_with_repos};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use domain::errors::DomainError;
use domain::ports::UserRepository;
use domain::testing::InMemoryStore;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

// Custom stub for SearchPort that returns empty results instead of panicking
struct SearchPortStub;
#[async_trait::async_trait]
impl domain::ports::SearchPort for SearchPortStub {
    async fn search(
        &self,
        _: &domain::models::SearchQuery,
    ) -> Result<domain::models::SearchResults, DomainError> {
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
    async fn get_by_id(
        &self,
        _: &domain::models::PersonId,
    ) -> Result<Option<domain::models::Person>, DomainError> {
        Ok(None) // Return None to trigger 404
    }
    async fn get_by_external_id(
        &self,
        _: &domain::models::ExternalPersonId,
    ) -> Result<Option<domain::models::Person>, DomainError> {
        Ok(None)
    }
    async fn get_credits(
        &self,
        _: &domain::models::PersonId,
    ) -> Result<domain::models::PersonCredits, DomainError> {
        Err(DomainError::NotFound("Person not found".into()))
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<domain::models::PersonId>, DomainError> {
        Ok(vec![])
    }
    async fn list_page(
        &self,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<domain::models::Person>, DomainError> {
        Ok(vec![])
    }
}

// --- Search endpoint tests ---

#[tokio::test]
async fn search_endpoint_returns_200_with_empty_results() {
    let state = crate::support::test_app_state_with_search_port(Arc::new(SearchPortStub));
    let app = Router::new()
        .route("/api/v1/search", get(http_axum::api::search::get_search))
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
    let state = crate::support::test_app_state_with_search_port(Arc::new(SearchPortStub));
    let app = Router::new()
        .route("/api/v1/search", get(http_axum::api::search::get_search))
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
    let state = crate::support::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}",
            get(http_axum::api::search::get_person_handler),
        )
        .with_state(state);

    let unknown_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/people/{}", unknown_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn person_credits_endpoint_returns_404_for_unknown_id() {
    let state = crate::support::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}/credits",
            get(http_axum::api::search::get_person_credits_handler),
        )
        .with_state(state);

    let unknown_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/people/{}/credits", unknown_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn injected_stub_reaches_prebuilt_deps() {
    let stub: Arc<dyn domain::ports::PersonQuery> = Arc::new(PersonQueryStub);
    let state = crate::support::test_app_state_with_person_query(Arc::clone(&stub));

    assert!(
        Arc::ptr_eq(&state.app_ctx.deps.person.get_person.person_query, &stub),
        "prebuilt deps must hold the injected stub too, or handlers reading deps \
         will silently use the real dependency"
    );
}

// --- Watchlist endpoint tests ---

#[tokio::test]
async fn get_watchlist_requires_auth() {
    let state = test_app_state();
    let app = Router::new()
        .route(
            "/api/v1/watchlist",
            get(http_axum::api::watchlist::get_watchlist_handler),
        )
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

// --- Pending-follower-count endpoint tests ---

struct FixedPendingCount(usize);
#[async_trait::async_trait]
impl domain::ports::FollowGraphQuery for FixedPendingCount {
    async fn get_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        unreachable!()
    }
    async fn get_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        unreachable!()
    }
    async fn get_pending_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        unreachable!()
    }
    async fn get_pending_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        unreachable!()
    }
    async fn count_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        unreachable!()
    }
    async fn count_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        unreachable!()
    }
    async fn count_pending_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(self.0)
    }
    async fn get_relation(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
    ) -> Result<domain::value_objects::FollowRelation, DomainError> {
        unreachable!()
    }
}

#[tokio::test]
async fn pending_count_endpoint_reports_the_use_case_result() {
    let uid = domain::value_objects::UserId::from_uuid(Uuid::new_v4());
    let users = InMemoryStore::new();
    users
        .save(&domain::testing::test_user_with_role(
            uid.clone(),
            domain::models::UserRole::Standard,
        ))
        .await
        .unwrap();
    let state = test_app_state_with_repos(
        |repos| {
            repos.follow_graph = Arc::new(FixedPendingCount(3));
            repos.user = users as _;
        },
        Arc::new(AcceptingAuth(uid)),
    );
    let app = Router::new()
        .route(
            "/api/v1/social/followers/pending/count",
            get(http_axum::api::social::get_pending_follower_count),
        )
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/social/followers/pending/count")
                .header("authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["count"], 3);
}

#[tokio::test]
async fn get_watchlist_status_requires_auth() {
    let state = test_app_state();
    let app = Router::new()
        .route(
            "/api/v1/watchlist/{movie_id}",
            get(http_axum::api::watchlist::get_watchlist_status),
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

// --- Federation collection error-handling tests ---

#[cfg(feature = "federation")]
struct FailingApDocument;

#[cfg(feature = "federation")]
#[async_trait::async_trait]
impl domain::ports::ApDocumentPort for FailingApDocument {
    async fn actor_json(&self, _: &str) -> Result<String, DomainError> {
        Err(DomainError::InfrastructureError("boom".into()))
    }
    async fn followers_collection_json(
        &self,
        _: Uuid,
        _: Option<u32>,
    ) -> Result<String, DomainError> {
        Err(DomainError::InfrastructureError("boom".into()))
    }
    async fn following_collection_json(
        &self,
        _: Uuid,
        _: Option<u32>,
    ) -> Result<String, DomainError> {
        Err(DomainError::InfrastructureError("boom".into()))
    }
}

#[cfg(feature = "federation")]
fn state_with_failing_ap_document() -> http_axum::state::AppState {
    let mut state = test_app_state();
    state.app_ctx.ap_document = Arc::new(FailingApDocument);
    state
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn followers_collection_returns_500_on_federation_error() {
    let app = http_axum::federation::html_routes().with_state(state_with_failing_ap_document());

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/users/{}/followers", Uuid::nil()))
                .header(axum::http::header::ACCEPT, "application/activity+json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn following_collection_returns_500_on_federation_error() {
    let app = http_axum::federation::html_routes().with_state(state_with_failing_ap_document());

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/users/{}/following", Uuid::nil()))
                .header(axum::http::header::ACCEPT, "application/activity+json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn refusal_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).expect("refusal body is JSON")
}

#[tokio::test]
async fn a_missing_resource_is_refused_with_not_found_code() {
    let state = crate::support::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}",
            get(http_axum::api::search::get_person_handler),
        )
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/people/{}", Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = refusal_body(resp).await;
    assert_eq!(body["code"], "NOT_FOUND");
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn a_missing_credential_is_refused_with_unauthorized_code() {
    let app = Router::new()
        .route(
            "/api/v1/watchlist",
            get(http_axum::api::watchlist::get_watchlist_handler),
        )
        .with_state(test_app_state());

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
    assert_eq!(refusal_body(resp).await["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn a_malformed_path_parameter_is_refused_with_invalid_input_code() {
    let state = crate::support::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}",
            get(http_axum::api::search::get_person_handler),
        )
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/people/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(refusal_body(resp).await["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn a_malformed_json_body_is_refused_with_invalid_input_code() {
    let app = Router::new()
        .route(
            "/api/v1/auth/login",
            axum::routing::post(http_axum::api::auth::login),
        )
        .with_state(test_app_state());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from("{not json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = refusal_body(resp).await;
    assert_eq!(body["code"], "INVALID_INPUT");
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn a_json_body_of_the_wrong_shape_is_refused_as_unprocessable() {
    let app = Router::new()
        .route(
            "/api/v1/auth/login",
            axum::routing::post(http_axum::api::auth::login),
        )
        .with_state(test_app_state());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refusal_body(resp).await["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn a_non_multipart_upload_is_refused_with_invalid_input_code() {
    let app = Router::new()
        .route(
            "/api/v1/webhooks/plex",
            axum::routing::post(http_axum::api::webhook::post_plex_webhook),
        )
        .with_state(test_app_state());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/plex?token=t")
                .header("content-type", "text/plain")
                .body(Body::from("hello"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_client_error());
    assert_eq!(refusal_body(resp).await["code"], "INVALID_INPUT");
}
