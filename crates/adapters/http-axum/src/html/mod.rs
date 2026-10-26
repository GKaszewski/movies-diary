pub mod csrf;
pub mod forms;
pub(crate) mod helpers;
mod mappers;
pub mod render;
pub mod templates;

mod auth;
mod diary;
mod images;
mod import;
mod integrations;
mod movies;
mod profile;
mod rss;
mod social;
mod tags;
mod users;
mod watchlist;
mod wrapup;

use axum::{Router, routing};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    let base = Router::new()
        .route("/", routing::get(diary::get_activity_feed_html))
        .route("/users", routing::get(users::get_users_list))
        .route("/u/{username}", routing::get(users::get_user_by_username))
        .route("/users/{id}", routing::get(profile::get_user_profile_html))
        .route(
            "/movies/{movie_id}",
            routing::get(movies::get_movie_detail_html),
        )
        .route("/reviews/new", routing::get(diary::get_new_review_page))
        .route("/reviews", routing::post(diary::post_review_html))
        .route(
            "/reviews/{id}/delete",
            routing::post(diary::post_delete_review_html),
        )
        .route("/images/{*key}", routing::get(images::get_image))
        .route(
            "/posters/{path}",
            routing::get(
                |axum::extract::Path(p): axum::extract::Path<String>| async move {
                    axum::response::Redirect::permanent(&format!("/images/{}", p))
                },
            ),
        )
        .route("/diary/export", routing::get(diary::get_export_html))
        .route("/import", routing::get(import::get_import_page))
        .route("/import/upload", routing::post(import::post_upload))
        .route(
            "/import/{id}/mapping",
            routing::get(import::get_mapping_page).post(import::post_mapping),
        )
        .route(
            "/import/{id}/preview",
            routing::get(import::get_preview_page),
        )
        .route("/import/{id}/confirm", routing::post(import::post_confirm))
        .route("/import/done", routing::get(import::get_import_done))
        .route(
            "/import/profiles/{profile_id}/delete",
            routing::post(import::post_delete_profile),
        )
        .route("/feed.rss", routing::get(rss::get_feed))
        .route("/users/{id}/feed.rss", routing::get(rss::get_user_feed))
        .route(
            "/settings/profile",
            routing::get(users::get_profile_settings).post(users::post_profile_settings),
        )
        .route(
            "/settings/account/delete",
            routing::post(users::post_delete_own_account),
        )
        .route("/users/{id}/delete", routing::post(users::post_delete_user))
        .route("/tags/{tag}", routing::get(tags::get_tag))
        .route(
            "/users/{id}/watchlist",
            routing::get(watchlist::get_watchlist_page),
        )
        .route(
            "/watchlist/add",
            routing::post(watchlist::post_watchlist_add_html),
        )
        .route(
            "/watchlist/{movie_id}/remove",
            routing::post(watchlist::post_watchlist_remove_html),
        )
        .route(
            "/settings/integrations",
            routing::get(integrations::get_integrations_page),
        )
        .route(
            "/settings/integrations/generate",
            routing::post(integrations::post_generate_token),
        )
        .route(
            "/settings/integrations/{id}/revoke",
            routing::post(integrations::post_revoke_token),
        )
        .route(
            "/watch-queue",
            routing::get(integrations::get_watch_queue_page),
        )
        .route(
            "/watch-queue/{id}/confirm",
            routing::post(integrations::post_confirm_single),
        )
        .route(
            "/watch-queue/{id}/dismiss",
            routing::post(integrations::post_dismiss_single),
        )
        .route(
            "/wrapups/{user_id}/{year}",
            routing::get(wrapup::get_user_wrapup_html),
        )
        .route(
            "/wrapups/global/{year}",
            routing::get(wrapup::get_global_wrapup_html),
        );

    base.merge(social_routes())
}

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route(
            "/login",
            routing::get(auth::get_login_page).post(auth::post_login),
        )
        .route("/logout", routing::get(auth::get_logout))
        .route(
            "/register",
            routing::get(auth::get_register_page).post(auth::post_register),
        )
}

fn social_routes() -> Router<AppState> {
    Router::new()
        .route("/users/{id}/follow", routing::post(social::follow_user))
        .route("/users/{id}/unfollow", routing::post(social::unfollow_user))
        .route(
            "/users/{id}/followers/accept",
            routing::post(social::accept_follower_html),
        )
        .route(
            "/users/{id}/followers/reject",
            routing::post(social::reject_follower_html),
        )
        .route(
            "/users/{id}/followers/remove",
            routing::post(social::remove_follower_html),
        )
        .route(
            "/users/{id}/following-list",
            routing::get(social::get_following_page),
        )
        .route(
            "/users/{id}/followers-list",
            routing::get(social::get_followers_page),
        )
}
