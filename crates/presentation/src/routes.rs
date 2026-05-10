use std::num::NonZeroU32;

use axum::{Router, routing};
use axum_governor::{GovernorConfigBuilder, GovernorLayer, Quota, extractor::PeerIp};
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{handlers, state::AppState};

pub fn build_router(state: AppState, ap_router: Router) -> Router {
    let rate_limit = state.app_ctx.config.rate_limit;

    let ap_cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit))
        .finish()
        .unwrap();
    let ap_router = ap_router.layer(GovernorLayer::new(ap_cfg));

    Router::new()
        .merge(html_routes(rate_limit))
        .merge(api_routes(rate_limit))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .merge(ap_router)
}

fn per_minute(n: u64) -> Quota {
    let n = NonZeroU32::new(n.clamp(1, u32::MAX as u64) as u32).unwrap();
    Quota::requests_per_minute(n)
}

fn html_routes(rate_limit: u64) -> Router<AppState> {
    let auth = Router::new()
        .route(
            "/login",
            routing::get(handlers::html::get_login_page).post(handlers::html::post_login),
        )
        .route("/logout", routing::get(handlers::html::get_logout))
        .route(
            "/register",
            routing::get(handlers::html::get_register_page).post(handlers::html::post_register),
        )
        .layer({
            let cfg = GovernorConfigBuilder::default()
                .with_extractor(PeerIp::default())
                .expect_connect_info()
                .quota_default(per_minute(rate_limit))
                .finish()
                .unwrap();
            GovernorLayer::new(cfg)
        });

    let base = Router::new()
        .route("/", routing::get(handlers::html::get_activity_feed))
        .route("/users", routing::get(handlers::html::get_users_list))
        .route(
            "/users/{id}",
            routing::get(handlers::html::get_user_profile),
        )
        .route(
            "/movies/{movie_id}",
            routing::get(handlers::html::get_movie_detail),
        )
        .merge(auth)
        .route(
            "/reviews/new",
            routing::get(handlers::html::get_new_review_page),
        )
        .route("/reviews", routing::post(handlers::html::post_review))
        .route(
            "/reviews/{id}/delete",
            routing::post(handlers::html::post_delete_review),
        )
        .route(
            "/posters/{*path}",
            routing::get(handlers::posters::get_poster),
        )
        .route("/diary/export", routing::get(handlers::html::get_export))
        .route("/import", routing::get(handlers::import::get_import_page))
        .route("/import/upload", routing::post(handlers::import::post_upload))
        .route("/import/{id}/mapping", routing::get(handlers::import::get_mapping_page).post(handlers::import::post_mapping))
        .route("/import/{id}/preview", routing::get(handlers::import::get_preview_page))
        .route("/import/{id}/confirm", routing::post(handlers::import::post_confirm))
        .route("/import/done", routing::get(handlers::import::get_import_done))
        .route("/import/profiles/{profile_id}/delete", routing::post(handlers::import::post_delete_profile))
        .route("/feed.rss", routing::get(handlers::rss::get_feed))
        .route(
            "/users/{id}/feed.rss",
            routing::get(handlers::rss::get_user_feed),
        );

    #[cfg(feature = "federation")]
    let base = base.merge(federation_html_routes());

    base.layer(axum::middleware::from_fn(crate::csrf::csrf_middleware))
}

#[cfg(feature = "federation")]
fn federation_html_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/users/{id}/follow",
            routing::post(handlers::html::follow_remote_user),
        )
        .route(
            "/users/{id}/unfollow",
            routing::post(handlers::html::unfollow_remote_user),
        )
        .route(
            "/users/{id}/followers/accept",
            routing::post(handlers::html::accept_follower),
        )
        .route(
            "/users/{id}/followers/reject",
            routing::post(handlers::html::reject_follower),
        )
        .route(
            "/users/{id}/following-list",
            routing::get(handlers::html::get_following_page),
        )
        .route(
            "/users/{id}/followers-list",
            routing::get(handlers::html::get_followers_page),
        )
        .route(
            "/users/{id}/followers/remove",
            routing::post(handlers::html::remove_follower),
        )
}

fn api_routes(rate_limit: u64) -> Router<AppState> {
    let cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit))
        .finish()
        .unwrap();

    let base = Router::new()
        .route("/diary", routing::get(handlers::api::get_diary))
        .route(
            "/movies/{id}/history",
            routing::get(handlers::api::get_review_history),
        )
        .route(
            "/movies/{id}",
            routing::get(handlers::api::get_movie_detail),
        )
        .route("/reviews", routing::post(handlers::api::post_review))
        .route(
            "/reviews/{id}",
            routing::delete(handlers::api::delete_review),
        )
        .route(
            "/movies/{id}/sync-poster",
            routing::post(handlers::api::sync_poster),
        )
        .route("/auth/login", routing::post(handlers::api::login))
        .route("/auth/register", routing::post(handlers::api::register))
        .route("/diary/export", routing::get(handlers::api::export_diary))
        .route(
            "/activity-feed",
            routing::get(handlers::api::get_activity_feed),
        )
        .route("/users", routing::get(handlers::api::list_users))
        .route("/users/{id}", routing::get(handlers::api::get_user_profile))
        .route("/import/sessions", routing::post(handlers::import::api_post_session))
        .route("/import/sessions/{id}", routing::get(handlers::import::api_get_session))
        .route("/import/sessions/{id}/mapping", routing::put(handlers::import::api_put_mapping))
        .route("/import/sessions/{id}/confirm", routing::post(handlers::import::api_post_confirm))
        .route("/import/profiles", routing::get(handlers::import::api_get_profiles).post(handlers::import::api_post_profile))
        .route("/import/profiles/{id}", routing::delete(handlers::import::api_delete_profile));

    #[cfg(feature = "federation")]
    let base = base.merge(federation_api_routes());

    Router::new().nest("/api/v1", base.layer(GovernorLayer::new(cfg)))
}

#[cfg(feature = "federation")]
fn federation_api_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/social/following",
            routing::get(handlers::api::get_following),
        )
        .route(
            "/social/followers",
            routing::get(handlers::api::get_followers),
        )
        .route(
            "/social/followers/pending",
            routing::get(handlers::api::get_pending_followers),
        )
        .route("/social/follow", routing::post(handlers::api::follow))
        .route("/social/unfollow", routing::post(handlers::api::unfollow))
        .route(
            "/social/followers/accept",
            routing::post(handlers::api::accept_follower),
        )
        .route(
            "/social/followers/reject",
            routing::post(handlers::api::reject_follower),
        )
        .route(
            "/social/followers/remove",
            routing::post(handlers::api::remove_follower),
        )
}
