const DEFAULT_PAGE_LIMIT: u32 = 5;
const RSS_FEED_LIMIT: u32 = 50;

pub mod html {
    use std::str::FromStr;

    use axum::{
        Form,
        extract::{Path, Query, State},
        http::{HeaderValue, StatusCode, header::SET_COOKIE},
        response::{Html, IntoResponse, Redirect},
    };
    use chrono::Utc;
    use uuid::Uuid;

    use application::{
        commands::{DeleteReviewCommand, ExportCommand, LoginCommand, RegisterCommand},
        ports::{
            FollowersPageData, FollowingPageData, HtmlPageContext, LoginPageData,
            NewReviewPageData, RegisterPageData, RemoteActorView,
        },
        use_cases::{
            delete_review, export_diary as export_diary_uc, log_review, login as login_uc,
            register as register_uc,
        },
    };
    use domain::models::ExportFormat;
    use domain::{errors::DomainError, value_objects::UserId};

    use crate::{
        dtos::{
            DiaryQueryParams, ErrorQuery, FollowForm, FollowerActionForm, LogReviewData,
            LogReviewForm, LoginForm, RegisterForm, UnfollowForm,
        },
        extractors::{OptionalCookieUser, RequiredCookieUser},
        state::AppState,
    };

    async fn build_page_context(state: &AppState, user_id: Option<UserId>) -> HtmlPageContext {
        let uuid = user_id.as_ref().map(|u| u.value());
        let user_email = if let Some(ref id) = user_id {
            state
                .app_ctx
                .user_repository
                .find_by_id(id)
                .await
                .ok()
                .flatten()
                .map(|u| u.email().value().to_string())
        } else {
            None
        };
        HtmlPageContext {
            user_email,
            user_id: uuid,
            register_enabled: state.app_ctx.config.allow_registration,
            rss_url: "/feed.rss".to_string(),
            page_title: "Movies Diary".to_string(),
            canonical_url: state.app_ctx.config.base_url.clone(),
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
    ) -> impl IntoResponse {
        let ctx = HtmlPageContext {
            user_email: None,
            user_id: None,
            register_enabled: state.app_ctx.config.allow_registration,
            rss_url: "/feed.rss".to_string(),
            page_title: "Login — Movies Diary".to_string(),
            canonical_url: format!("{}/login", state.app_ctx.config.base_url),
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
        Form(form): Form<LoginForm>,
    ) -> impl IntoResponse {
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
    ) -> impl IntoResponse {
        if !state.app_ctx.config.allow_registration {
            return Redirect::to("/").into_response();
        }
        let ctx = HtmlPageContext {
            user_email: None,
            user_id: None,
            register_enabled: true,
            rss_url: "/feed.rss".to_string(),
            page_title: "Register — Movies Diary".to_string(),
            canonical_url: format!("{}/register", state.app_ctx.config.base_url),
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
        Form(form): Form<RegisterForm>,
    ) -> impl IntoResponse {
        if !state.app_ctx.config.allow_registration {
            return Redirect::to("/").into_response();
        }
        let email = form.email.clone();
        let password = form.password.clone();
        match register_uc::execute(
            &state.app_ctx,
            RegisterCommand {
                email: form.email,
                username: form.username,
                password: form.password,
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
    ) -> impl IntoResponse {
        let mut ctx = build_page_context(&state, Some(user_id)).await;
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
        Form(form): Form<LogReviewForm>,
    ) -> impl IntoResponse {
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
        Path(review_id): Path<Uuid>,
        Form(form): Form<crate::dtos::DeleteRedirectForm>,
    ) -> impl IntoResponse {
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
        Query(params): Query<crate::dtos::ExportQueryParams>,
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
        Query(params): Query<DiaryQueryParams>,
    ) -> impl IntoResponse {
        let ctx = build_page_context(&state, user_id).await;
        let query = application::queries::GetActivityFeedQuery {
            limit: params.limit,
            offset: params.offset,
        };
        match application::use_cases::get_activity_feed::execute(&state.app_ctx, query).await {
            Ok(entries) => {
                let limit = entries.limit;
                let offset = entries.offset;
                let has_more = (offset as u64).saturating_add(limit as u64) < entries.total_count;
                let data = application::ports::ActivityFeedPageData {
                    ctx,
                    current_offset: offset,
                    has_more,
                    limit,
                    entries,
                };
                match state.html_renderer.render_activity_feed_page(data) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
                }
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    pub async fn get_users_list(
        OptionalCookieUser(user_id): OptionalCookieUser,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        let mut ctx = build_page_context(&state, user_id).await;
        ctx.page_title = "Members — Movies Diary".to_string();
        ctx.canonical_url = format!("{}/users", state.app_ctx.config.base_url);
        match application::use_cases::get_users::execute(
            &state.app_ctx,
            application::queries::GetUsersQuery,
        )
        .await
        {
            Ok(users) => {
                let data = application::ports::UsersPageData { ctx, users };
                match state.html_renderer.render_users_page(data) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
                }
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    pub async fn get_user_profile(
        OptionalCookieUser(user_id): OptionalCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        headers: axum::http::HeaderMap,
        Query(params): Query<crate::dtos::ProfileQueryParams>,
    ) -> impl IntoResponse {
        // Content negotiation: AP clients request application/activity+json
        let accept = headers
            .get(axum::http::header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
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

        let mut ctx = build_page_context(&state, user_id.clone()).await;
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

        let is_own_profile = user_id
            .as_ref()
            .map(|u| u.value() == profile_user_uuid)
            .unwrap_or(false);

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

        let followers_count = if is_own_profile {
            state
                .ap_service
                .count_accepted_followers(profile_user_uuid)
                .await
                .unwrap_or(0)
        } else {
            0
        };

        let pending_followers = if is_own_profile {
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
                })
                .collect()
        } else {
            vec![]
        };

        let query = application::queries::GetUserProfileQuery {
            user_id: profile_user_uuid,
            view: profile_view,
            limit: params.limit,
            offset: params.offset,
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
                ctx.rss_url = format!("/users/{}/feed.rss", profile_user_uuid);
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
                };
                match state.html_renderer.render_profile_page(data) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
                }
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    pub async fn follow_remote_user(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Form(form): Form<FollowForm>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
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

    pub async fn unfollow_remote_user(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Form(form): Form<UnfollowForm>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
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

    pub async fn accept_follower(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Form(form): Form<FollowerActionForm>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
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

    pub async fn reject_follower(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Form(form): Form<FollowerActionForm>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
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

    pub async fn get_following_page(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Query(params): Query<crate::dtos::ErrorQuery>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
            return StatusCode::FORBIDDEN.into_response();
        }
        let mut ctx = build_page_context(&state, Some(user_id.clone())).await;
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

    pub async fn get_followers_page(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Query(params): Query<crate::dtos::ErrorQuery>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
            return StatusCode::FORBIDDEN.into_response();
        }
        let mut ctx = build_page_context(&state, Some(user_id.clone())).await;
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

    pub async fn remove_follower(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Path(profile_user_uuid): Path<Uuid>,
        Form(form): Form<FollowerActionForm>,
    ) -> impl IntoResponse {
        if user_id.value() != profile_user_uuid {
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
}

pub mod posters {
    use axum::{
        extract::{Path, State},
        http::{StatusCode, header},
        response::IntoResponse,
    };

    use domain::value_objects::PosterPath;

    use crate::state::AppState;

    pub async fn get_poster(
        State(state): State<AppState>,
        Path(path): Path<String>,
    ) -> impl IntoResponse {
        // If path is a remote URL, redirect directly instead of serving from local storage.
        if path.starts_with("http://") || path.starts_with("https://") {
            return axum::response::Redirect::temporary(&path).into_response();
        }

        let poster_path = match PosterPath::new(path) {
            Ok(p) => p,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        match state.app_ctx.poster_storage.get_poster(&poster_path).await {
            Ok(bytes) => {
                let mime = infer::get(&bytes)
                    .map(|t| t.mime_type())
                    .unwrap_or("application/octet-stream");
                ([(header::CONTENT_TYPE, mime)], bytes).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

pub mod rss {
    use axum::{
        extract::{Path, State},
        http::header,
        response::IntoResponse,
    };
    use uuid::Uuid;

    use application::{queries::GetDiaryQuery, use_cases::get_diary};
    use domain::{errors::DomainError, models::SortDirection, value_objects::UserId};

    use crate::{errors::ApiError, state::AppState};

    pub async fn get_feed(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
        let query = GetDiaryQuery {
            limit: Some(super::RSS_FEED_LIMIT),
            offset: Some(0),
            sort_by: Some(SortDirection::Descending),
            movie_id: None,
            user_id: None,
        };
        let page = get_diary::execute(&state.app_ctx, query).await?;
        let xml = state
            .rss_renderer
            .render_feed(&page.items, "Movie Diary")
            .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;
        Ok((
            [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
            xml,
        ))
    }

    pub async fn get_user_feed(
        State(state): State<AppState>,
        Path(user_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, ApiError> {
        let user = state
            .app_ctx
            .user_repository
            .find_by_id(&UserId::from_uuid(user_id))
            .await
            .map_err(ApiError)?
            .ok_or_else(|| ApiError(DomainError::NotFound(format!("User {user_id}"))))?;

        let query = GetDiaryQuery {
            limit: Some(super::RSS_FEED_LIMIT),
            offset: Some(0),
            sort_by: Some(SortDirection::Descending),
            movie_id: None,
            user_id: Some(user_id),
        };
        let page = get_diary::execute(&state.app_ctx, query).await?;

        let display_name = user.email().value().split('@').next().unwrap_or("User");
        let title = format!("{}'s Movie Diary", display_name);

        let xml = state
            .rss_renderer
            .render_feed(&page.items, &title)
            .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;

        Ok((
            [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
            xml,
        ))
    }
}

pub mod api {
    use axum::{
        Json,
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
    };
    use uuid::Uuid;

    use application::{
        commands::{DeleteReviewCommand, LoginCommand, RegisterCommand, SyncPosterCommand},
        queries::GetReviewHistoryQuery,
        use_cases::{
            delete_review, get_diary, get_review_history, log_review, login as login_uc,
            register as register_uc, sync_poster,
        },
    };
    use domain::{
        errors::DomainError,
        models::{DiaryEntry, Movie, Review},
        services::review_history::Trend,
        value_objects::MovieId,
    };

    use crate::{
        dtos::{
            DiaryEntryDto, DiaryQueryParams, DiaryResponse, LogReviewData, LogReviewRequest,
            LoginRequest, LoginResponse, MovieDto, RegisterRequest, ReviewDto,
            ReviewHistoryResponse,
        },
        errors::ApiError,
        extractors::AuthenticatedUser,
        state::AppState,
    };

    pub async fn get_diary(
        State(state): State<AppState>,
        Query(params): Query<DiaryQueryParams>,
    ) -> Result<Json<DiaryResponse>, ApiError> {
        let page = get_diary::execute(&state.app_ctx, params.into()).await?;

        Ok(Json(DiaryResponse {
            items: page.items.iter().map(entry_to_dto).collect(),
            total_count: page.total_count,
            limit: page.limit,
            offset: page.offset,
        }))
    }

    pub async fn get_review_history(
        State(state): State<AppState>,
        Path(movie_id): Path<Uuid>,
    ) -> Result<Json<ReviewHistoryResponse>, ApiError> {
        let (history, trend) =
            get_review_history::execute(&state.app_ctx, GetReviewHistoryQuery { movie_id }).await?;

        Ok(Json(ReviewHistoryResponse {
            movie: movie_to_dto(history.movie()),
            viewings: history.viewings().iter().map(review_to_dto).collect(),
            trend: match trend {
                Trend::Improved => "improved",
                Trend::Declined => "declined",
                Trend::Neutral => "neutral",
            }
            .to_string(),
        }))
    }

    pub async fn post_review(
        State(state): State<AppState>,
        user: AuthenticatedUser,
        Json(req): Json<LogReviewRequest>,
    ) -> Result<impl IntoResponse, ApiError> {
        let data = LogReviewData::try_from(req).map_err(ApiError)?;
        log_review::execute(&state.app_ctx, data.into_command(user.0.value())).await?;
        Ok(StatusCode::CREATED)
    }

    pub async fn sync_poster(
        State(state): State<AppState>,
        _user: AuthenticatedUser,
        Path(movie_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, ApiError> {
        let movie = state
            .app_ctx
            .movie_repository
            .get_movie_by_id(&MovieId::from_uuid(movie_id))
            .await?
            .ok_or_else(|| ApiError(DomainError::NotFound(format!("Movie {movie_id}"))))?;

        let external_id = movie
            .external_metadata_id()
            .ok_or_else(|| {
                ApiError(DomainError::ValidationError(
                    "Movie has no external metadata ID, cannot sync poster".into(),
                ))
            })?
            .value()
            .to_string();

        sync_poster::execute(
            &state.app_ctx,
            SyncPosterCommand {
                movie_id,
                external_metadata_id: external_id,
            },
        )
        .await?;

        Ok(StatusCode::NO_CONTENT)
    }

    pub async fn login(
        State(state): State<AppState>,
        Json(req): Json<LoginRequest>,
    ) -> Result<Json<LoginResponse>, ApiError> {
        let result = login_uc::execute(
            &state.app_ctx,
            LoginCommand {
                email: req.email,
                password: req.password,
            },
        )
        .await?;
        Ok(Json(LoginResponse {
            token: result.token,
            user_id: result.user_id,
            email: result.email,
            expires_at: result.expires_at.to_rfc3339(),
        }))
    }

    pub async fn register(
        State(state): State<AppState>,
        Json(req): Json<RegisterRequest>,
    ) -> Result<StatusCode, ApiError> {
        register_uc::execute(
            &state.app_ctx,
            RegisterCommand {
                email: req.email,
                username: req.username,
                password: req.password,
            },
        )
        .await?;
        Ok(StatusCode::CREATED)
    }

    pub async fn delete_review(
        State(state): State<AppState>,
        AuthenticatedUser(user_id): AuthenticatedUser,
        Path(review_id): Path<Uuid>,
    ) -> impl IntoResponse {
        let cmd = DeleteReviewCommand {
            review_id,
            requesting_user_id: user_id.value(),
        };
        match delete_review::execute(&state.app_ctx, cmd).await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
            Err(e) => {
                tracing::error!("delete_review error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }

    fn movie_to_dto(movie: &Movie) -> MovieDto {
        MovieDto {
            id: movie.id().value(),
            title: movie.title().value().to_string(),
            release_year: movie.release_year().value(),
            director: movie.director().map(|d| d.to_string()),
            poster_path: movie.poster_path().map(|p| p.value().to_string()),
        }
    }

    fn review_to_dto(review: &Review) -> ReviewDto {
        ReviewDto {
            id: review.id().value(),
            rating: review.rating().value(),
            comment: review.comment().map(|c| c.value().to_string()),
            watched_at: review.watched_at().to_string(),
        }
    }

    fn entry_to_dto(entry: &DiaryEntry) -> DiaryEntryDto {
        DiaryEntryDto {
            movie: movie_to_dto(entry.movie()),
            review: review_to_dto(entry.review()),
        }
    }
}
