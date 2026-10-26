pub mod auth;
pub mod diary;
pub mod goals;
pub mod import;
pub mod insights;
pub mod movies;
pub mod search;
pub mod social;
pub mod user_profile;
pub mod users;
pub mod watchlist;
pub mod webhook;
pub mod wrapup;

use axum::{Router, routing};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    let base = Router::new()
        .route("/diary", routing::get(diary::get_diary))
        .route(
            "/movies/{id}/history",
            routing::get(movies::get_review_history),
        )
        .route("/movies", routing::get(movies::list_movies))
        .route("/movies/{id}", routing::get(movies::get_movie_detail))
        .route(
            "/movies/{id}/profile",
            routing::get(movies::get_movie_profile),
        )
        .route("/reviews", routing::post(diary::post_review))
        .route(
            "/reviews/{id}",
            routing::delete(diary::delete_review).patch(diary::patch_review),
        )
        .route(
            "/movies/{id}/sync-poster",
            routing::post(movies::sync_poster),
        )
        .route("/auth/login", routing::post(auth::login))
        .route("/auth/register", routing::post(auth::register))
        .route("/auth/refresh", routing::post(auth::refresh))
        .route("/auth/logout", routing::post(auth::api_logout))
        .route("/diary/export", routing::get(diary::export_diary))
        .route("/activity-feed", routing::get(diary::get_activity_feed))
        .route("/users", routing::get(users::list_users))
        .route("/users/me", routing::delete(users::delete_me))
        .route(
            "/users/{id}",
            routing::get(user_profile::get_user_profile).delete(users::delete_user),
        )
        .route("/import/sessions", routing::post(import::api_post_session))
        .route(
            "/import/sessions/{id}",
            routing::get(import::api_get_session),
        )
        .route(
            "/import/sessions/{id}/mapping",
            routing::put(import::api_put_mapping),
        )
        .route(
            "/import/sessions/{id}/preview",
            routing::get(import::api_get_preview),
        )
        .route(
            "/import/sessions/{id}/confirm",
            routing::post(import::api_post_confirm),
        )
        .route(
            "/import/profiles",
            routing::get(import::api_get_profiles).post(import::api_post_profile),
        )
        .route(
            "/import/profiles/{id}",
            routing::delete(import::api_delete_profile),
        )
        .route(
            "/import/sessions/{id}/profile/{profile_id}",
            routing::put(import::api_apply_profile),
        )
        .route(
            "/profile",
            routing::get(users::get_profile).put(users::update_profile_handler),
        )
        .route(
            "/profile/fields",
            routing::put(users::update_profile_fields_handler),
        )
        .route("/search", routing::get(search::get_search))
        .route("/people/{id}", routing::get(search::get_person_handler))
        .route(
            "/people/{id}/credits",
            routing::get(search::get_person_credits_handler),
        )
        .route(
            "/watchlist",
            routing::get(watchlist::get_watchlist_handler).post(watchlist::post_watchlist_add),
        )
        .route(
            "/watchlist/{movie_id}",
            routing::get(watchlist::get_watchlist_status).delete(watchlist::delete_watchlist_entry),
        )
        .route(
            "/settings/webhook-tokens",
            routing::get(webhook::get_webhook_tokens).post(webhook::post_generate_webhook_token),
        )
        .route(
            "/settings/webhook-tokens/{id}",
            routing::delete(webhook::delete_webhook_token),
        )
        .route("/watch-queue", routing::get(webhook::get_watch_queue))
        .route(
            "/watch-queue/confirm",
            routing::post(webhook::post_confirm_watch_events),
        )
        .route(
            "/watch-queue/dismiss",
            routing::post(webhook::post_dismiss_watch_events),
        )
        .route("/wrapups/generate", routing::post(wrapup::post_generate))
        .route("/wrapups", routing::get(wrapup::get_list))
        .route(
            "/wrapups/{id}",
            routing::get(wrapup::get_status).delete(wrapup::delete_wrapup_handler),
        )
        .route("/wrapups/{id}/report", routing::get(wrapup::get_report))
        .route("/insights", routing::get(insights::get_insights))
        .route(
            "/admin/reindex-search",
            routing::post(search::post_reindex_search),
        )
        .route(
            "/goals",
            routing::get(goals::list_goals).post(goals::create_goal),
        )
        .route(
            "/goals/{year}",
            routing::put(goals::update_goal).delete(goals::delete_goal),
        )
        .route("/users/{id}/goals", routing::get(goals::get_user_goals))
        .route(
            "/settings",
            routing::get(goals::get_settings).put(goals::update_settings),
        );

    base.merge(social_routes())
}

pub fn webhook_router() -> Router<AppState> {
    Router::new()
        .route(
            "/webhooks/jellyfin",
            routing::post(webhook::post_jellyfin_webhook),
        )
        .route("/webhooks/plex", routing::post(webhook::post_plex_webhook))
}

fn social_routes() -> Router<AppState> {
    Router::new()
        .route("/social/following", routing::get(social::get_following))
        .route("/social/followers", routing::get(social::get_followers))
        .route(
            "/social/followers/pending",
            routing::get(social::get_pending_followers),
        )
        .route(
            "/social/followers/pending/count",
            routing::get(social::get_pending_follower_count),
        )
        .route(
            "/social/following/pending",
            routing::get(social::get_pending_following),
        )
        .route(
            "/social/relationship",
            routing::get(social::get_relationship),
        )
        .route("/social/follow", routing::post(social::follow))
        .route("/social/unfollow", routing::post(social::unfollow))
        .route(
            "/social/followers/accept",
            routing::post(social::accept_follower),
        )
        .route(
            "/social/followers/reject",
            routing::post(social::reject_follower),
        )
        .route(
            "/social/followers/remove",
            routing::post(social::remove_follower),
        )
        .route(
            "/users/{id}/following",
            routing::get(social::get_user_following),
        )
        .route(
            "/users/{id}/followers",
            routing::get(social::get_user_followers),
        )
}
