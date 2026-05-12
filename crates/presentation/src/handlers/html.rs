use std::str::FromStr;

use axum::{
    Form,
    extract::{Extension, Multipart, Path, Query, State},
    http::{HeaderValue, StatusCode, header::SET_COOKIE},
    response::{Html, IntoResponse, Redirect},
};
use chrono::Utc;
use uuid::Uuid;

#[cfg(feature = "federation")]
use application::ports::{
    BlockedActorEntry, BlockedActorsPageData, BlockedDomainEntry, BlockedDomainsPageData,
    FollowersPageData, FollowingPageData,
};
use application::{
    commands::{DeleteReviewCommand, ExportCommand, LoginCommand, RegisterCommand},
    ports::{
        HtmlPageContext, LoginPageData, MovieDetailPageData, NewReviewPageData,
        ProfileSettingsPageData, RegisterPageData, RemoteActorView,
    },
    queries::GetMovieSocialPageQuery,
    use_cases::{
        delete_review, export_diary as export_diary_uc, get_movie_social_page, log_review,
        login as login_uc, register as register_uc, update_profile,
    },
};
use domain::models::ExportFormat;
use domain::{errors::DomainError, value_objects::UserId};

#[cfg(feature = "federation")]
use crate::forms::{ActorUrlForm, BlockDomainForm, FollowForm, FollowerActionForm, RemoveDomainForm, UnfollowForm};
use crate::{
    csrf::CsrfToken,
    forms::{ErrorQuery, FeedQueryParams, LogReviewData, LogReviewForm, LoginForm, RegisterForm},
    extractors::{AdminUser, OptionalCookieUser, RequiredCookieUser},
    state::AppState,
};

pub(crate) async fn build_page_context(
    state: &AppState,
    user_id: Option<UserId>,
    csrf_token: String,
) -> HtmlPageContext {
    let uuid = user_id.as_ref().map(|u| u.value());
    let (user_email, is_admin) = if let Some(ref id) = user_id {
        let user = state
            .app_ctx
            .user_repository
            .find_by_id(id)
            .await
            .ok()
            .flatten();
        let email = user.as_ref().map(|u| u.email().value().to_string());
        let admin = user.as_ref().map(|u| matches!(u.role(), domain::models::UserRole::Admin)).unwrap_or(false);
        (email, admin)
    } else {
        (None, false)
    };
    HtmlPageContext {
        user_email,
        user_id: uuid,
        is_admin,
        register_enabled: state.app_ctx.config.allow_registration,
        rss_url: "/feed.rss".to_string(),
        page_title: "Movies Diary".to_string(),
        canonical_url: state.app_ctx.config.base_url.clone(),
        csrf_token,
        page_rss_url: None,
    }
}

fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

fn secure_flag() -> &'static str {
    if std::env::var("SECURE_COOKIES").as_deref() == Ok("true") {
        "; Secure"
    } else {
        ""
    }
}

fn set_cookie_header(token: &str, max_age: i64) -> (axum::http::HeaderName, HeaderValue) {
    let val = format!(
        "token={}; HttpOnly; Path=/; SameSite=Strict; Max-Age={}{}",
        token,
        max_age,
        secure_flag()
    );
    (
        SET_COOKIE,
        HeaderValue::from_str(&val).expect("valid cookie"),
    )
}

pub async fn get_login_page(
    State(state): State<AppState>,
    Query(params): Query<ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = HtmlPageContext {
        user_email: None,
        user_id: None,
        is_admin: false,
        register_enabled: state.app_ctx.config.allow_registration,
        rss_url: "/feed.rss".to_string(),
        page_title: "Login — Movies Diary".to_string(),
        canonical_url: format!("{}/login", state.app_ctx.config.base_url),
        csrf_token: csrf.0,
        page_rss_url: None,
    };
    let html = state
        .html_renderer
        .render_login_page(LoginPageData {
            ctx,
            error: params.error.as_deref(),
        })
        .expect("login template failed");
    Html(html)
}

pub async fn post_login(
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match login_uc::execute(
        &state.app_ctx,
        LoginCommand {
            email: form.email,
            password: form.password,
        },
    )
    .await
    {
        Ok(result) => {
            let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
            let cookie = set_cookie_header(&result.token, max_age);
            ([cookie], Redirect::to("/")).into_response()
        }
        Err(_) => Redirect::to("/login?error=Invalid+credentials").into_response(),
    }
}

pub async fn get_logout() -> impl IntoResponse {
    let val = format!(
        "token=; HttpOnly; Path=/; SameSite=Strict; Max-Age=0{}",
        secure_flag()
    );
    let cookie = (
        SET_COOKIE,
        HeaderValue::from_str(&val).expect("valid cookie"),
    );
    ([cookie], Redirect::to("/")).into_response()
}

pub async fn get_register_page(
    State(state): State<AppState>,
    Query(params): Query<ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    if !state.app_ctx.config.allow_registration {
        return Redirect::to("/").into_response();
    }
    let ctx = HtmlPageContext {
        user_email: None,
        user_id: None,
        is_admin: false,
        register_enabled: true,
        rss_url: "/feed.rss".to_string(),
        page_title: "Register — Movies Diary".to_string(),
        canonical_url: format!("{}/register", state.app_ctx.config.base_url),
        csrf_token: csrf.0,
        page_rss_url: None,
    };
    let html = state
        .html_renderer
        .render_register_page(RegisterPageData {
            ctx,
            error: params.error.as_deref(),
        })
        .expect("register template failed");
    Html(html).into_response()
}

pub async fn post_register(
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    if !state.app_ctx.config.allow_registration {
        return Redirect::to("/").into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let email = form.email.clone();
    let password = form.password.clone();
    match register_uc::execute(
        &state.app_ctx,
        RegisterCommand {
            email: form.email,
            username: form.username,
            password: form.password,
            role: domain::models::UserRole::Standard,
        },
    )
    .await
    {
        Ok(_) => {
            match login_uc::execute(&state.app_ctx, LoginCommand { email, password }).await {
                Ok(result) => {
                    let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
                    let cookie = set_cookie_header(&result.token, max_age);
                    ([cookie], Redirect::to("/")).into_response()
                }
                Err(_) => Redirect::to("/login").into_response(),
            }
        }
        Err(_) => Redirect::to("/register?error=Registration+failed.+Please+try+again.")
            .into_response(),
    }
}

pub async fn get_new_review_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id), csrf.0).await;
    ctx.page_title = "Log a Review — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/reviews/new", state.app_ctx.config.base_url);
    let html = state
        .html_renderer
        .render_new_review_page(NewReviewPageData {
            ctx,
            error: params.error.as_deref(),
        })
        .expect("new_review template failed");
    Html(html)
}

pub async fn post_review(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<LogReviewForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let data = match LogReviewData::try_from(form) {
        Ok(d) => d,
        Err(_) => {
            return Redirect::to("/reviews/new?error=Invalid+date+format").into_response();
        }
    };

    match log_review::execute(&state.app_ctx, data.into_command(user_id.value())).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/reviews/new?error={}", msg)).into_response()
        }
    }
}

pub async fn post_delete_review(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Path(review_id): Path<Uuid>,
    Form(form): Form<crate::forms::DeleteRedirectForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    match delete_review::execute(&state.app_ctx, cmd).await {
        Ok(()) => {
            let redirect_url = form
                .redirect_after
                .filter(|url| {
                    (url.starts_with('/') && !url.starts_with("//")) || url.starts_with('?')
                })
                .unwrap_or_else(|| "/".to_string());
            Redirect::to(&redirect_url).into_response()
        }
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => {
            tracing::error!("delete_review html error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_export(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Query(params): Query<api_types::ExportQueryParams>,
) -> impl IntoResponse {
    let format = match params.format.as_str() {
        "csv" => ExportFormat::Csv,
        "json" => ExportFormat::Json,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (content_type, filename) = match &format {
        ExportFormat::Csv => ("text/csv; charset=utf-8", "diary.csv"),
        ExportFormat::Json => ("application/json", "diary.json"),
    };
    let cmd = ExportCommand {
        user_id: user_id.value(),
        format,
    };
    match export_diary_uc::execute(&state.app_ctx, cmd).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, content_type.to_string()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => {
            tracing::error!("export error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_activity_feed(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Query(params): Query<FeedQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    #[cfg(feature = "federation")]
    let filter_str = if params.filter == "following" && user_id.is_some() {
        "following"
    } else {
        "all"
    };
    #[cfg(not(feature = "federation"))]
    let filter_str = "all";

    let sort_by_str = match params.sort_by.as_str() {
        "date_asc" => "date_asc",
        "rating" => "rating",
        "rating_asc" => "rating_asc",
        _ => "date",
    };

    #[cfg(feature = "federation")]
    let following = if filter_str == "following" {
        if let Some(uid) = user_id {
            let urls = state
                .social_query
                .get_accepted_following_urls(uid.value())
                .await
                .unwrap_or_default();
            let base_url = &state.app_ctx.config.base_url;
            let mut local_ids = vec![uid.value()];
            let mut remote_urls = Vec::new();
            for url in urls {
                if let Some(suffix) = url.strip_prefix(&format!("{}/users/", base_url)) {
                    if let Ok(parsed_id) = uuid::Uuid::parse_str(suffix) {
                        local_ids.push(parsed_id);
                        continue;
                    }
                }
                remote_urls.push(url);
            }
            Some(domain::ports::FollowingFilter {
                local_user_ids: local_ids,
                remote_actor_urls: remote_urls,
            })
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(not(feature = "federation"))]
    let following: Option<domain::ports::FollowingFilter> = None;

    let search_opt = if params.search.is_empty() {
        None
    } else {
        Some(params.search.clone())
    };

    let query = application::queries::GetActivityFeedQuery {
        limit,
        offset,
        sort_by: domain::ports::FeedSortBy::from_str(sort_by_str),
        search: search_opt,
        following,
    };

    match application::use_cases::get_activity_feed::execute(&state.app_ctx, query).await {
        Ok(entries) => {
            let entry_limit = entries.limit;
            let entry_offset = entries.offset;
            let has_more =
                (entry_offset as u64).saturating_add(entry_limit as u64) < entries.total_count;
            let data = application::ports::ActivityFeedPageData {
                ctx,
                current_offset: entry_offset,
                has_more,
                limit: entry_limit,
                entries,
                filter: filter_str.to_string(),
                sort_by: sort_by_str.to_string(),
                search: params.search,
            };
            match state.html_renderer.render_activity_feed_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_users_list(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, user_id, csrf.0).await;
    ctx.page_title = "Members — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/users", state.app_ctx.config.base_url);

    #[cfg(feature = "federation")]
    let (users_result, actors_result) = tokio::join!(
        application::use_cases::get_users::execute(
            &state.app_ctx,
            application::queries::GetUsersQuery,
        ),
        state.social_query.list_all_followed_remote_actors()
    );
    #[cfg(not(feature = "federation"))]
    let (users_result, actors_result) = (
        application::use_cases::get_users::execute(
            &state.app_ctx,
            application::queries::GetUsersQuery,
        )
        .await,
        Ok::<Vec<domain::ports::RemoteActorInfo>, domain::errors::DomainError>(vec![]),
    );

    match (users_result, actors_result) {
        (Ok(users), Ok(remote_actors)) => {
            let actor_views = remote_actors
                .into_iter()
                .map(|a| application::ports::RemoteActorView {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                    avatar_url: None,
                })
                .collect();
            let data = application::ports::UsersPageData {
                ctx,
                users,
                remote_actors: actor_views,
            };
            match state.html_renderer.render_users_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        (Err(e), _) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        (_, Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_user_profile(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<crate::forms::ProfileQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    // Content negotiation: AP clients request application/activity+json
    #[cfg(feature = "federation")]
    {
        let accept = headers
            .get(axum::http::header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if accept.contains("application/activity+json")
            || accept.contains("application/ld+json")
        {
            return match state
                .ap_service
                .actor_json(&profile_user_uuid.to_string())
                .await
            {
                Ok(json) => (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "application/activity+json",
                    )],
                    json,
                )
                    .into_response(),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            };
        }
    }

    let mut ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let view_str = params.view.as_deref().unwrap_or("recent");
    let profile_view = match application::queries::ProfileView::from_str(view_str) {
        Ok(v) => v,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "invalid view parameter",
            )
                .into_response();
        }
    };

    let profile_user = match state
        .app_ctx
        .user_repository
        .find_by_id(&domain::value_objects::UserId::from_uuid(profile_user_uuid))
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let display_name = profile_user.username().value();
    ctx.page_title = format!("{}'s Diary — Movies Diary", display_name);
    ctx.canonical_url = format!(
        "{}/users/{}",
        state.app_ctx.config.base_url, profile_user_uuid
    );

    let sort_by_str = match params.sort_by.as_str() {
        "date_asc" => "date_asc",
        "rating" => "rating",
        "rating_asc" => "rating_asc",
        _ => "date",
    };

    let is_own_profile = user_id
        .as_ref()
        .map(|u| u.value() == profile_user_uuid)
        .unwrap_or(false);

    #[cfg(feature = "federation")]
    let following_count = if is_own_profile {
        if let Some(ref uid) = user_id {
            state
                .ap_service
                .count_following(uid.value())
                .await
                .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };
    #[cfg(not(feature = "federation"))]
    let following_count = 0usize;

    #[cfg(feature = "federation")]
    let followers_count = if is_own_profile {
        state
            .ap_service
            .count_accepted_followers(profile_user_uuid)
            .await
            .unwrap_or(0)
    } else {
        0
    };
    #[cfg(not(feature = "federation"))]
    let followers_count = 0usize;

    #[cfg(feature = "federation")]
    let pending_followers: Vec<application::ports::RemoteActorView> = if is_own_profile {
        state
            .ap_service
            .get_pending_followers(profile_user_uuid)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|a| application::ports::RemoteActorView {
                handle: a.handle,
                url: a.url,
                display_name: a.display_name,
                avatar_url: a.avatar_url.clone(),
            })
            .collect()
    } else {
        vec![]
    };
    #[cfg(not(feature = "federation"))]
    let pending_followers: Vec<application::ports::RemoteActorView> = vec![];

    let query = application::queries::GetUserProfileQuery {
        user_id: profile_user_uuid,
        view: profile_view,
        limit: params.limit,
        offset: params.offset,
        sort_by: domain::ports::FeedSortBy::from_str(sort_by_str),
        search: if params.search.is_empty() {
            None
        } else {
            Some(params.search.clone())
        },
    };

    match application::use_cases::get_user_profile::execute(&state.app_ctx, query).await {
        Ok(profile) => {
            let (offset, has_more, limit) = profile
                .entries
                .as_ref()
                .map(|e| {
                    let has_more =
                        (e.offset as u64).saturating_add(e.limit as u64) < e.total_count;
                    (e.offset, has_more, e.limit)
                })
                .unwrap_or((0, false, super::DEFAULT_PAGE_LIMIT));
            if !is_own_profile {
                ctx.page_rss_url = Some(format!("/users/{}/feed.rss", profile_user_uuid));
            }
            let data = application::ports::ProfilePageData {
                ctx,
                profile_user_id: profile_user_uuid,
                profile_user_email: profile_user.email().value().to_string(),
                stats: profile.stats,
                view: profile_view.as_str().to_string(),
                entries: profile.entries,
                current_offset: offset,
                has_more,
                limit,
                history: profile.history,
                trends: profile.trends,
                is_own_profile,
                error: params.error,
                following_count,
                followers_count,
                pending_followers,
                sort_by: sort_by_str.to_string(),
                search: params.search.clone(),
            };
            match state.html_renderer.render_profile_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[cfg(feature = "federation")]
pub async fn follow_remote_user(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state.ap_service.follow(user_id.value(), &form.handle).await {
        Ok(()) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            tracing::error!("follow error: {:?}", e);
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn unfollow_remote_user(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<UnfollowForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .unfollow(user_id.value(), &form.actor_url)
        .await
    {
        Ok(()) => Redirect::to(&format!("/users/{}/following-list", profile_user_uuid))
            .into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!(
                "/users/{}/following-list?error={}",
                profile_user_uuid, msg
            ))
            .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn accept_follower(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .accept_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn reject_follower(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .reject_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_following_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Query(params): Query<crate::forms::ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Following — Movies Diary".to_string();
    ctx.canonical_url = format!(
        "{}/users/{}/following-list",
        state.app_ctx.config.base_url, profile_user_uuid
    );
    match state.ap_service.get_following(user_id.value()).await {
        Ok(following) => {
            let actors = following
                .into_iter()
                .map(|a| RemoteActorView {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                    avatar_url: a.avatar_url.clone(),
                })
                .collect();
            let data = FollowingPageData {
                ctx,
                user_id: profile_user_uuid,
                actors,
                error: params.error,
            };
            match state.html_renderer.render_following_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        Err(e) => {
            tracing::error!("get_following error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load following list",
            )
                .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_followers_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Query(params): Query<crate::forms::ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Followers — Movies Diary".to_string();
    ctx.canonical_url = format!(
        "{}/users/{}/followers-list",
        state.app_ctx.config.base_url, profile_user_uuid
    );
    match state
        .ap_service
        .get_accepted_followers(user_id.value())
        .await
    {
        Ok(followers) => {
            let actors = followers
                .into_iter()
                .map(|a| RemoteActorView {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                    avatar_url: a.avatar_url.clone(),
                })
                .collect();
            let data = FollowersPageData {
                ctx,
                user_id: profile_user_uuid,
                actors,
                error: params.error,
            };
            match state.html_renderer.render_followers_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        Err(e) => {
            tracing::error!("get_followers error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load followers list",
            )
                .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn remove_follower(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .remove_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}/followers-list", profile_user_uuid))
            .into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!(
                "/users/{}/followers-list?error={}",
                profile_user_uuid, msg
            ))
            .into_response()
        }
    }
}

pub async fn get_movie_detail(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(movie_id): Path<uuid::Uuid>,
    Query(params): Query<api_types::PaginationQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, user_id, csrf.0).await;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    match get_movie_social_page::execute(
        &state.app_ctx,
        GetMovieSocialPageQuery { movie_id, limit, offset },
    )
    .await
    {
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(DomainError::ValidationError(_)) => StatusCode::BAD_REQUEST.into_response(),
        Err(e) => {
            tracing::error!("movie detail error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(result) => {
            let histogram_max = result.stats.rating_histogram.iter().copied().max().unwrap_or(1);
            let has_more = result.reviews.offset + result.reviews.limit
                < result.reviews.total_count as u32;
            let data = MovieDetailPageData {
                ctx,
                movie: result.movie,
                stats: result.stats,
                current_offset: result.reviews.offset,
                has_more,
                limit: result.reviews.limit,
                reviews: result.reviews,
                histogram_max,
            };
            match state.html_renderer.render_movie_detail_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("template error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct SavedQuery {
    pub saved: Option<String>,
}

pub async fn get_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<SavedQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Profile Settings — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/settings/profile", state.app_ctx.config.base_url);

    let user = match state
        .app_ctx
        .user_repository
        .find_by_id(&user_id)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_profile_settings user lookup: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let base_url = &state.app_ctx.config.base_url;
    let avatar_url = user
        .avatar_path()
        .map(|path| format!("{}/images/{}", base_url, path));

    let saved = params.saved.as_deref() == Some("1");

    let data = ProfileSettingsPageData {
        ctx,
        bio: user.bio().map(|s| s.to_string()),
        avatar_url,
        saved,
    };

    match state.html_renderer.render_profile_settings_page(data) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("profile_settings template error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_tag(Path(tag): Path<String>) -> impl IntoResponse {
    if tag.eq_ignore_ascii_case("moviesdiary") {
        Redirect::temporary("/")
    } else {
        Redirect::temporary(&format!("/?search={}", tag))
    }
}

#[cfg(feature = "federation")]
pub async fn get_blocked_domains_page(
    AdminUser(user_id): AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id), csrf.0).await;
    ctx.page_title = "Blocked Domains — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/admin/blocked-domains", state.app_ctx.config.base_url);
    match state.ap_service.get_blocked_domains().await {
        Ok(domains) => {
            let data = BlockedDomainsPageData {
                ctx,
                domains: domains
                    .into_iter()
                    .map(|d| BlockedDomainEntry {
                        domain: d.domain,
                        reason: d.reason,
                        blocked_at: d.blocked_at,
                    })
                    .collect(),
            };
            match state.html_renderer.render_blocked_domains_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        Err(e) => {
            tracing::error!("get_blocked_domains error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load blocked domains").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_blocked_domain(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<BlockDomainForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let reason = form.reason.as_deref().filter(|s| !s.trim().is_empty());
    match state.ap_service.add_blocked_domain(&form.domain, reason).await {
        Ok(()) => Redirect::to("/admin/blocked-domains").into_response(),
        Err(e) => {
            tracing::error!("add_blocked_domain error: {:?}", e);
            Redirect::to("/admin/blocked-domains").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_remove_blocked_domain(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<RemoveDomainForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state.ap_service.remove_blocked_domain(&form.domain).await {
        Ok(()) => Redirect::to("/admin/blocked-domains").into_response(),
        Err(e) => {
            tracing::error!("remove_blocked_domain error: {:?}", e);
            Redirect::to("/admin/blocked-domains").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_blocked_actors_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Blocked Users — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/social/blocked", state.app_ctx.config.base_url);
    match state.ap_service.get_blocked_actors(user_id.value()).await {
        Ok(actors) => {
            let data = BlockedActorsPageData {
                ctx,
                actors: actors
                    .into_iter()
                    .map(|a| BlockedActorEntry {
                        url: a.url,
                        handle: a.handle,
                        display_name: a.display_name,
                        avatar_url: a.avatar_url,
                    })
                    .collect(),
            };
            match state.html_renderer.render_blocked_actors_page(data) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        Err(e) => {
            tracing::error!("get_blocked_actors error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load blocked users").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_block_actor_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state.ap_service.block_actor(user_id.value(), &form.actor_url).await {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("block_actor html error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_unblock_actor(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state.ap_service.unblock_actor(user_id.value(), &form.actor_url).await {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("unblock_actor error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}

pub async fn post_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut bio: Option<String> = None;
    let mut avatar_bytes: Option<Vec<u8>> = None;
    let mut avatar_content_type: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "bio" => {
                if let Ok(text) = field.text().await {
                    bio = Some(text);
                }
            }
            "avatar" => {
                let content_type = field.content_type().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await {
                    if !bytes.is_empty() {
                        avatar_bytes = Some(bytes.to_vec());
                        avatar_content_type = content_type;
                    }
                }
            }
            _ => {}
        }
    }

    let cmd = update_profile::UpdateProfileCommand {
        user_id: user_id.value(),
        bio,
        avatar_bytes,
        avatar_content_type,
    };

    let _ = update_profile::execute(&state.app_ctx, cmd).await;

    Redirect::to("/settings/profile?saved=1").into_response()
}
