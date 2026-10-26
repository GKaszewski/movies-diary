#[cfg(feature = "federation")]
pub(crate) mod blocklist_api;
#[cfg(feature = "federation")]
mod blocklist_html;
#[cfg(feature = "federation")]
pub(crate) mod collections;
#[cfg(feature = "federation")]
mod negotiation;

#[cfg(feature = "federation")]
pub use negotiation::actor_document_response;

use axum::{Router, routing};

use crate::state::AppState;

#[cfg(feature = "federation")]
pub fn html_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/users/{id}/followers",
            routing::get(collections::get_followers_collection),
        )
        .route(
            "/users/{id}/following",
            routing::get(collections::get_following_collection),
        )
        .route(
            "/admin/blocked-domains",
            routing::get(blocklist_html::get_blocked_domains_page)
                .post(blocklist_html::post_blocked_domain),
        )
        .route(
            "/admin/blocked-domains/remove",
            routing::post(blocklist_html::post_remove_blocked_domain),
        )
        .route(
            "/social/blocked",
            routing::get(blocklist_html::get_blocked_actors_page),
        )
        .route(
            "/social/block",
            routing::post(blocklist_html::post_block_actor_html),
        )
        .route(
            "/social/unblock",
            routing::post(blocklist_html::post_unblock_actor),
        )
}

#[cfg(feature = "federation")]
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/blocked-domains",
            routing::get(blocklist_api::get_blocked_domains_admin)
                .post(blocklist_api::add_blocked_domain_admin),
        )
        .route(
            "/admin/blocked-domains/{domain}",
            routing::delete(blocklist_api::remove_blocked_domain_admin),
        )
        .route(
            "/social/block",
            routing::post(blocklist_api::block_actor_api),
        )
        .route(
            "/social/unblock",
            routing::post(blocklist_api::unblock_actor_api),
        )
        .route(
            "/social/blocked",
            routing::get(blocklist_api::get_blocked_actors_api),
        )
}

#[cfg(not(feature = "federation"))]
pub fn html_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/users/{id}/followers",
            routing::get(
                |axum::extract::Path(id): axum::extract::Path<uuid::Uuid>| async move {
                    axum::response::Redirect::permanent(&format!("/users/{}/followers-list", id))
                },
            ),
        )
        .route(
            "/users/{id}/following",
            routing::get(
                |axum::extract::Path(id): axum::extract::Path<uuid::Uuid>| async move {
                    axum::response::Redirect::permanent(&format!("/users/{}/following-list", id))
                },
            ),
        )
}

#[cfg(not(feature = "federation"))]
pub fn api_routes() -> Router<AppState> {
    Router::new()
}

#[cfg(not(feature = "federation"))]
pub async fn actor_document_response(
    _state: &AppState,
    _headers: &axum::http::HeaderMap,
    _user_id: uuid::Uuid,
) -> Option<axum::response::Response> {
    None
}
