use crate::extractors::tests::{AcceptingAuth, test_app_state, test_app_state_with_follow_graph};
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
    let state = crate::extractors::tests::test_app_state_with_search_port(Arc::new(SearchPortStub));
    let app = Router::new()
        .route("/api/v1/search", get(crate::handlers::search::get_search))
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
    let state = crate::extractors::tests::test_app_state_with_search_port(Arc::new(SearchPortStub));
    let app = Router::new()
        .route("/api/v1/search", get(crate::handlers::search::get_search))
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
    let state =
        crate::extractors::tests::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}",
            get(crate::handlers::search::get_person_handler),
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
    let state =
        crate::extractors::tests::test_app_state_with_person_query(Arc::new(PersonQueryStub));
    let app = Router::new()
        .route(
            "/api/v1/people/{id}/credits",
            get(crate::handlers::search::get_person_credits_handler),
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

/// Proves stub injection reaches the prebuilt deps, not just the `Repositories`
/// passed into `build_test_state_from`. Before the fix that added construct-time
/// stub injection, this failed on the assertion below — exactly the false-green
/// that would have silently broken the person-handler tests in Task 6. `AppContext`
/// no longer carries a `repos` field (Plan C2), so the original companion
/// assertion on `state.app_ctx.repos.person_query` was dropped along with the
/// field; the assertion that matters — that `deps` itself holds the stub — stays.
#[test]
fn injected_stub_reaches_prebuilt_deps() {
    let stub: Arc<dyn domain::ports::PersonQuery> = Arc::new(PersonQueryStub);
    let state = crate::extractors::tests::test_app_state_with_person_query(Arc::clone(&stub));

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
            get(crate::handlers::watchlist::get_watchlist_handler),
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

/// Stub `FollowGraphQuery` returning a fixed pending-follower count.
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

/// The point of Task 4: one use case (`social::count_pending_followers`) now
/// serves both the classic UI (via `get_page_viewer`) and this new HTTP
/// endpoint. Seed the query port with a fixed count and assert the endpoint
/// reflects it verbatim.
#[tokio::test]
async fn pending_count_endpoint_reports_the_use_case_result() {
    let uid = domain::value_objects::UserId::from_uuid(Uuid::new_v4());
    let state = test_app_state_with_follow_graph(
        Arc::new(FixedPendingCount(3)),
        Arc::new(AcceptingAuth(uid)),
    );
    let app = Router::new()
        .route(
            "/api/v1/social/followers/pending/count",
            get(crate::handlers::social::get_pending_follower_count),
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
            get(crate::handlers::watchlist::get_watchlist_status),
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

/// Swapping `ap_document` on the returned state is safe — unlike the `repos`
/// fields, handlers read `app_ctx.ap_document` directly and no prebuilt deps
/// struct carries a copy of it.
#[cfg(feature = "federation")]
fn state_with_failing_ap_document() -> crate::state::AppState {
    let mut state = test_app_state();
    state.app_ctx.ap_document = Arc::new(FailingApDocument);
    state
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn followers_collection_returns_500_on_federation_error() {
    let app = Router::new()
        .route(
            "/users/{id}/followers",
            get(crate::handlers::social::get_followers_collection),
        )
        .with_state(state_with_failing_ap_document());

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
    let app = Router::new()
        .route(
            "/users/{id}/following",
            get(crate::handlers::social::get_following_collection),
        )
        .with_state(state_with_failing_ap_document());

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
