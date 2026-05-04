use axum::{Router, routing};
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(html_routes())
        .merge(api_routes())
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn html_routes() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(handlers::html::get_index))
        .route(
            "/login",
            routing::get(handlers::html::get_login_page)
                .post(handlers::html::post_login),
        )
        .route("/logout", routing::get(handlers::html::get_logout))
        .route(
            "/register",
            routing::get(handlers::html::get_register_page)
                .post(handlers::html::post_register),
        )
        .route("/reviews/new", routing::get(handlers::html::get_new_review_page))
        .route("/reviews", routing::post(handlers::html::post_review))
        .route("/feed.rss", routing::get(handlers::rss::get_feed))
}

fn api_routes() -> Router<AppState> {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/diary", routing::get(handlers::api::get_diary))
            .route(
                "/movies/{id}/history",
                routing::get(handlers::api::get_review_history),
            )
            .route("/reviews", routing::post(handlers::api::post_review))
            .route(
                "/movies/{id}/sync-poster",
                routing::post(handlers::api::sync_poster),
            )
            .route("/auth/login", routing::post(handlers::api::login))
            .route("/auth/register", routing::post(handlers::api::register)),
    )
}
