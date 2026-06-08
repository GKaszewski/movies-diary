use std::num::NonZeroU32;

use axum::{Router, routing};
use axum_governor::{GovernorConfigBuilder, GovernorLayer, Quota, extractor::PeerIp};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{handlers, state::AppState};

pub fn build_router(state: AppState, ap_router: Router) -> Router {
    let rate_limit = state.app_ctx.config.rate_limit;

    let ap_cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit / 2))
        .finish()
        .unwrap();
    let ap_router = ap_router.layer(GovernorLayer::new(ap_cfg));

    Router::new()
        .route("/health", routing::get(health_handler))
        .merge(html_routes(rate_limit))
        .merge(api_routes(rate_limit))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/app",
            ServeDir::new("spa/dist").fallback(ServeFile::new("spa/dist/index.html")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .merge(ap_router)
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

fn per_minute(n: u64) -> Quota {
    let n = NonZeroU32::new(n.clamp(1, u32::MAX as u64) as u32).unwrap();
    Quota::requests_per_minute(n)
}

fn html_routes(rate_limit: u64) -> Router<AppState> {
    let auth = Router::new()
        .route(
            "/login",
            routing::get(handlers::auth::get_login_page).post(handlers::auth::post_login),
        )
        .route("/logout", routing::get(handlers::auth::get_logout))
        .route(
            "/register",
            routing::get(handlers::auth::get_register_page).post(handlers::auth::post_register),
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
        .route("/", routing::get(handlers::diary::get_activity_feed_html))
        .route("/users", routing::get(handlers::users::get_users_list))
        .route(
            "/u/{username}",
            routing::get(handlers::users::get_user_by_username),
        )
        .route(
            "/users/{id}",
            routing::get(handlers::users::get_user_profile_html),
        )
        .route(
            "/movies/{movie_id}",
            routing::get(handlers::movies::get_movie_detail_html),
        )
        .merge(auth)
        .route(
            "/reviews/new",
            routing::get(handlers::diary::get_new_review_page),
        )
        .route("/reviews", routing::post(handlers::diary::post_review_html))
        .route(
            "/reviews/{id}/delete",
            routing::post(handlers::diary::post_delete_review_html),
        )
        .route("/images/{*key}", routing::get(handlers::images::get_image))
        .route(
            "/posters/{path}",
            routing::get(
                |axum::extract::Path(p): axum::extract::Path<String>| async move {
                    axum::response::Redirect::permanent(&format!("/images/{}", p))
                },
            ),
        )
        .route(
            "/diary/export",
            routing::get(handlers::diary::get_export_html),
        )
        .route("/import", routing::get(handlers::import::get_import_page))
        .route(
            "/import/upload",
            routing::post(handlers::import::post_upload),
        )
        .route(
            "/import/{id}/mapping",
            routing::get(handlers::import::get_mapping_page).post(handlers::import::post_mapping),
        )
        .route(
            "/import/{id}/preview",
            routing::get(handlers::import::get_preview_page),
        )
        .route(
            "/import/{id}/confirm",
            routing::post(handlers::import::post_confirm),
        )
        .route(
            "/import/done",
            routing::get(handlers::import::get_import_done),
        )
        .route(
            "/import/profiles/{profile_id}/delete",
            routing::post(handlers::import::post_delete_profile),
        )
        .route("/feed.rss", routing::get(handlers::rss::get_feed))
        .route(
            "/users/{id}/feed.rss",
            routing::get(handlers::rss::get_user_feed),
        )
        .route(
            "/settings/profile",
            routing::get(handlers::users::get_profile_settings)
                .post(handlers::users::post_profile_settings),
        )
        .route("/tags/{tag}", routing::get(handlers::search::get_tag))
        .route(
            "/users/{id}/watchlist",
            routing::get(handlers::watchlist::get_watchlist_page),
        )
        .route(
            "/watchlist/add",
            routing::post(handlers::watchlist::post_watchlist_add_html),
        )
        .route(
            "/watchlist/{movie_id}/remove",
            routing::post(handlers::watchlist::post_watchlist_remove_html),
        )
        .route(
            "/settings/integrations",
            routing::get(handlers::integrations::get_integrations_page),
        )
        .route(
            "/settings/integrations/generate",
            routing::post(handlers::integrations::post_generate_token),
        )
        .route(
            "/settings/integrations/{id}/revoke",
            routing::post(handlers::integrations::post_revoke_token),
        )
        .route(
            "/watch-queue",
            routing::get(handlers::integrations::get_watch_queue_page),
        )
        .route(
            "/watch-queue/{id}/confirm",
            routing::post(handlers::integrations::post_confirm_single),
        )
        .route(
            "/watch-queue/{id}/dismiss",
            routing::post(handlers::integrations::post_dismiss_single),
        )
        .route(
            "/wrapups/{user_id}/{year}",
            routing::get(handlers::wrapup::get_user_wrapup_html),
        )
        .route(
            "/wrapups/global/{year}",
            routing::get(handlers::wrapup::get_global_wrapup_html),
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
            routing::post(handlers::social::follow_remote_user),
        )
        .route(
            "/users/{id}/unfollow",
            routing::post(handlers::social::unfollow_remote_user),
        )
        .route(
            "/users/{id}/followers/accept",
            routing::post(handlers::social::accept_follower_html),
        )
        .route(
            "/users/{id}/followers/reject",
            routing::post(handlers::social::reject_follower_html),
        )
        .route(
            "/users/{id}/followers",
            routing::get(handlers::social::get_followers_collection),
        )
        .route(
            "/users/{id}/following",
            routing::get(handlers::social::get_following_collection),
        )
        .route(
            "/users/{id}/following-list",
            routing::get(handlers::social::get_following_page),
        )
        .route(
            "/users/{id}/followers-list",
            routing::get(handlers::social::get_followers_page),
        )
        .route(
            "/users/{id}/followers/remove",
            routing::post(handlers::social::remove_follower_html),
        )
        .route(
            "/admin/blocked-domains",
            routing::get(handlers::social::get_blocked_domains_page)
                .post(handlers::social::post_blocked_domain),
        )
        .route(
            "/admin/blocked-domains/remove",
            routing::post(handlers::social::post_remove_blocked_domain),
        )
        .route(
            "/social/blocked",
            routing::get(handlers::social::get_blocked_actors_page),
        )
        .route(
            "/social/block",
            routing::post(handlers::social::post_block_actor_html),
        )
        .route(
            "/social/unblock",
            routing::post(handlers::social::post_unblock_actor),
        )
}

fn cors_layer() -> CorsLayer {
    use axum::http::{HeaderName, Method};
    use tower_http::cors::AllowOrigin;

    let origins = std::env::var("CORS_ORIGINS").unwrap_or_default();
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ]);

    if origins.is_empty() || origins == "*" {
        layer.allow_origin(AllowOrigin::any())
    } else {
        let parsed: Vec<_> = origins
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                match trimmed.parse() {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("ignoring invalid CORS origin {trimmed:?}: {e}");
                        None
                    }
                }
            })
            .collect();
        layer
            .allow_origin(AllowOrigin::list(parsed))
            .allow_credentials(true)
    }
}

fn api_routes(rate_limit: u64) -> Router<AppState> {
    let cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit))
        .finish()
        .unwrap();

    let base = Router::new()
        .route("/diary", routing::get(handlers::diary::get_diary))
        .route(
            "/movies/{id}/history",
            routing::get(handlers::movies::get_review_history),
        )
        .route("/movies", routing::get(handlers::movies::list_movies))
        .route(
            "/movies/{id}",
            routing::get(handlers::movies::get_movie_detail),
        )
        .route(
            "/movies/{id}/profile",
            routing::get(handlers::movies::get_movie_profile),
        )
        .route("/reviews", routing::post(handlers::diary::post_review))
        .route(
            "/reviews/{id}",
            routing::delete(handlers::diary::delete_review),
        )
        .route(
            "/movies/{id}/sync-poster",
            routing::post(handlers::movies::sync_poster),
        )
        .route("/auth/login", routing::post(handlers::auth::login))
        .route("/auth/register", routing::post(handlers::auth::register))
        .route("/diary/export", routing::get(handlers::diary::export_diary))
        .route(
            "/activity-feed",
            routing::get(handlers::diary::get_activity_feed),
        )
        .route("/users", routing::get(handlers::users::list_users))
        .route(
            "/users/{id}",
            routing::get(handlers::users::get_user_profile),
        )
        .route(
            "/import/sessions",
            routing::post(handlers::import::api_post_session),
        )
        .route(
            "/import/sessions/{id}",
            routing::get(handlers::import::api_get_session),
        )
        .route(
            "/import/sessions/{id}/mapping",
            routing::put(handlers::import::api_put_mapping),
        )
        .route(
            "/import/sessions/{id}/preview",
            routing::get(handlers::import::api_get_preview),
        )
        .route(
            "/import/sessions/{id}/confirm",
            routing::post(handlers::import::api_post_confirm),
        )
        .route(
            "/import/profiles",
            routing::get(handlers::import::api_get_profiles)
                .post(handlers::import::api_post_profile),
        )
        .route(
            "/import/profiles/{id}",
            routing::delete(handlers::import::api_delete_profile),
        )
        .route(
            "/profile",
            routing::get(handlers::users::get_profile).put(handlers::users::update_profile_handler),
        )
        .route(
            "/profile/fields",
            routing::put(handlers::users::update_profile_fields_handler),
        )
        .route("/search", routing::get(handlers::search::get_search))
        .route(
            "/people/{id}",
            routing::get(handlers::search::get_person_handler),
        )
        .route(
            "/people/{id}/credits",
            routing::get(handlers::search::get_person_credits_handler),
        )
        .route(
            "/watchlist",
            routing::get(handlers::watchlist::get_watchlist_handler)
                .post(handlers::watchlist::post_watchlist_add),
        )
        .route(
            "/watchlist/{movie_id}",
            routing::get(handlers::watchlist::get_watchlist_status)
                .delete(handlers::watchlist::delete_watchlist_entry),
        )
        .route(
            "/settings/webhook-tokens",
            routing::get(handlers::webhook::get_webhook_tokens)
                .post(handlers::webhook::post_generate_webhook_token),
        )
        .route(
            "/settings/webhook-tokens/{id}",
            routing::delete(handlers::webhook::delete_webhook_token),
        )
        .route(
            "/watch-queue",
            routing::get(handlers::webhook::get_watch_queue),
        )
        .route(
            "/watch-queue/confirm",
            routing::post(handlers::webhook::post_confirm_watch_events),
        )
        .route(
            "/watch-queue/dismiss",
            routing::post(handlers::webhook::post_dismiss_watch_events),
        )
        .route(
            "/wrapups/generate",
            routing::post(handlers::wrapup::post_generate),
        )
        .route("/wrapups", routing::get(handlers::wrapup::get_list))
        .route(
            "/wrapups/{id}",
            routing::get(handlers::wrapup::get_status)
                .delete(handlers::wrapup::delete_wrapup_handler),
        )
        .route(
            "/wrapups/{id}/report",
            routing::get(handlers::wrapup::get_report),
        )
        .route(
            "/admin/reindex-search",
            routing::post(handlers::search::post_reindex_search),
        )
        .route(
            "/goals",
            routing::get(handlers::goals::list_goals).post(handlers::goals::create_goal),
        )
        .route(
            "/goals/{year}",
            routing::put(handlers::goals::update_goal).delete(handlers::goals::delete_goal),
        )
        .route(
            "/users/{id}/goals",
            routing::get(handlers::goals::get_user_goals),
        )
        .route(
            "/settings",
            routing::get(handlers::goals::get_settings).put(handlers::goals::update_settings),
        );

    #[cfg(feature = "federation")]
    let base = base.merge(federation_api_routes());

    let webhook_cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit / 4))
        .finish()
        .unwrap();
    let webhook_routes = Router::new()
        .route(
            "/webhooks/jellyfin",
            routing::post(handlers::webhook::post_jellyfin_webhook),
        )
        .route(
            "/webhooks/plex",
            routing::post(handlers::webhook::post_plex_webhook),
        )
        .layer(GovernorLayer::new(webhook_cfg));

    Router::new()
        .nest("/api/v1", base.layer(GovernorLayer::new(cfg)))
        .nest("/api/v1", webhook_routes)
        .layer(cors_layer())
}

#[cfg(feature = "federation")]
fn federation_api_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/social/following",
            routing::get(handlers::social::get_following),
        )
        .route(
            "/social/followers",
            routing::get(handlers::social::get_followers),
        )
        .route(
            "/social/followers/pending",
            routing::get(handlers::social::get_pending_followers),
        )
        .route("/social/follow", routing::post(handlers::social::follow))
        .route(
            "/social/unfollow",
            routing::post(handlers::social::unfollow),
        )
        .route(
            "/social/followers/accept",
            routing::post(handlers::social::accept_follower),
        )
        .route(
            "/social/followers/reject",
            routing::post(handlers::social::reject_follower),
        )
        .route(
            "/social/followers/remove",
            routing::post(handlers::social::remove_follower),
        )
        .route(
            "/admin/blocked-domains",
            routing::get(handlers::social::get_blocked_domains_admin)
                .post(handlers::social::add_blocked_domain_admin),
        )
        .route(
            "/admin/blocked-domains/{domain}",
            routing::delete(handlers::social::remove_blocked_domain_admin),
        )
        .route(
            "/social/block",
            routing::post(handlers::social::block_actor_api),
        )
        .route(
            "/social/unblock",
            routing::post(handlers::social::unblock_actor_api),
        )
        .route(
            "/social/blocked",
            routing::get(handlers::social::get_blocked_actors_api),
        )
        .route(
            "/users/{id}/following",
            routing::get(handlers::social::get_user_following),
        )
        .route(
            "/users/{id}/followers",
            routing::get(handlers::social::get_user_followers),
        )
}
