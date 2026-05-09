use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{Router, http::StatusCode, middleware, response::IntoResponse, routing};
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{handlers, state::AppState};

/// Simple global rate limiter: tracks request count per 60-second window.
/// Not per-IP — suitable for a low-traffic personal app.
#[derive(Clone)]
struct RateLimiter {
    window: Arc<AtomicU64>,
    count: Arc<AtomicU64>,
    limit: u64,
}

impl RateLimiter {
    fn new(limit: u64) -> Self {
        Self {
            window: Arc::new(AtomicU64::new(0)),
            count: Arc::new(AtomicU64::new(0)),
            limit,
        }
    }

    fn check(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 60;
        let prev = self.window.load(Ordering::Acquire);
        if now != prev {
            // compare_exchange ensures only one thread wins the window reset
            if self
                .window
                .compare_exchange(prev, now, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.count.store(1, Ordering::Release);
                return true;
            }
        }
        self.count.fetch_add(1, Ordering::Relaxed) + 1 <= self.limit
    }
}

pub fn build_router(state: AppState, ap_router: Router) -> Router {
    let rate_limit = state.app_ctx.config.rate_limit;
    Router::new()
        .merge(html_routes(rate_limit))
        .merge(api_routes(rate_limit))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .merge(ap_router)
}

fn html_routes(rate_limit: u64) -> Router<AppState> {
    let limiter = RateLimiter::new(rate_limit);
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
        .route_layer(middleware::from_fn(
            move |req: axum::extract::Request, next: middleware::Next| {
                let limiter = limiter.clone();
                async move {
                    if limiter.check() {
                        next.run(req).await
                    } else {
                        StatusCode::TOO_MANY_REQUESTS.into_response()
                    }
                }
            },
        ));

    Router::new()
        .route("/", routing::get(handlers::html::get_activity_feed))
        .route("/users", routing::get(handlers::html::get_users_list))
        .route(
            "/users/{id}",
            routing::get(handlers::html::get_user_profile),
        )
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
        .route("/feed.rss", routing::get(handlers::rss::get_feed))
        .route(
            "/users/{id}/feed.rss",
            routing::get(handlers::rss::get_user_feed),
        )
}

fn api_routes(rate_limit: u64) -> Router<AppState> {
    let limiter = RateLimiter::new(rate_limit);
    let auth_rate_limit =
        middleware::from_fn(move |req: axum::extract::Request, next: middleware::Next| {
            let limiter = limiter.clone();
            async move {
                if limiter.check() {
                    next.run(req).await
                } else {
                    StatusCode::TOO_MANY_REQUESTS.into_response()
                }
            }
        });

    Router::new().nest(
        "/api/v1",
        Router::new()
            .route("/diary", routing::get(handlers::api::get_diary))
            .route(
                "/movies/{id}/history",
                routing::get(handlers::api::get_review_history),
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
            .route("/social/following", routing::get(handlers::api::get_following))
            .route("/social/followers", routing::get(handlers::api::get_followers))
            .route("/social/follow", routing::post(handlers::api::follow))
            .route("/social/unfollow", routing::post(handlers::api::unfollow))
            .route("/social/followers/accept", routing::post(handlers::api::accept_follower))
            .route("/social/followers/reject", routing::post(handlers::api::reject_follower))
            .route("/social/followers/remove", routing::post(handlers::api::remove_follower))
            .route_layer(auth_rate_limit),
    )
}
