use axum::{
    Form, Json,
    extract::{Extension, Query, State},
    http::{HeaderValue, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
};
use chrono::Utc;

use application::auth::{
    commands::RegisterCommand,
    deps::{LoginDeps, RefreshDeps, RegisterAndLoginDeps, RegisterDeps},
    login as login_uc,
    queries::LoginQuery,
    register as register_uc,
};

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    forms::{ErrorQuery, LoginForm, RegisterForm},
    render::render_page,
    state::AppState,
};
use api_types::HtmlPageContext;
use api_types::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
};
use template_askama::{LoginTemplate, RegisterTemplate};

// ── HTML helpers ─────────────────────────────────────────────────────────────

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

// ── API ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let deps = LoginDeps {
        user: state.app_ctx.repos.user.clone(),
        password_hasher: state.app_ctx.services.password_hasher.clone(),
        auth: state.app_ctx.services.auth.clone(),
        refresh_session: state.app_ctx.repos.refresh_session.clone(),
        config: state.app_ctx.config.clone(),
    };
    let result = login_uc::execute(
        &deps,
        LoginQuery {
            email: req.email,
            password: req.password,
        },
    )
    .await?;
    Ok(Json(LoginResponse {
        token: result.token,
        refresh_token: result.refresh_token,
        user_id: result.user_id,
        email: result.email,
        expires_at: result.expires_at.to_rfc3339(),
        role: result.role,
    }))
}

#[utoipa::path(
    post, path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<StatusCode, ApiError> {
    let deps = RegisterDeps {
        user: state.app_ctx.repos.user.clone(),
        password_hasher: state.app_ctx.services.password_hasher.clone(),
        config: state.app_ctx.config.clone(),
    };
    register_uc::execute(
        &deps,
        RegisterCommand {
            email: req.email,
            username: req.username,
            password: req.password,
            role: domain::models::UserRole::Standard,
        },
    )
    .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    post, path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, body = RefreshResponse),
        (status = 401, description = "Invalid or expired refresh token"),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let deps = RefreshDeps {
        refresh_session: state.app_ctx.repos.refresh_session.clone(),
        auth: state.app_ctx.services.auth.clone(),
        config: state.app_ctx.config.clone(),
    };
    let result = application::auth::refresh::execute(&deps, &req.refresh_token).await?;
    Ok(Json(RefreshResponse {
        token: result.token,
        refresh_token: result.refresh_token,
        expires_at: result.expires_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 204, description = "Logged out"),
    )
)]
pub async fn api_logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> StatusCode {
    let _ = application::auth::logout::execute(
        state.app_ctx.repos.refresh_session.clone(),
        &req.refresh_token,
    )
    .await;
    StatusCode::NO_CONTENT
}

// ── HTML ─────────────────────────────────────────────────────────────────────

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
    render_page(LoginTemplate {
        ctx: &ctx,
        error: params.error.as_deref(),
    })
}

pub async fn post_login(
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let deps = LoginDeps {
        user: state.app_ctx.repos.user.clone(),
        password_hasher: state.app_ctx.services.password_hasher.clone(),
        auth: state.app_ctx.services.auth.clone(),
        refresh_session: state.app_ctx.repos.refresh_session.clone(),
        config: state.app_ctx.config.clone(),
    };
    match login_uc::execute(
        &deps,
        LoginQuery {
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
    render_page(RegisterTemplate {
        ctx: &ctx,
        error: params.error.as_deref(),
    })
    .into_response()
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
    let deps = RegisterAndLoginDeps {
        user: state.app_ctx.repos.user.clone(),
        password_hasher: state.app_ctx.services.password_hasher.clone(),
        auth: state.app_ctx.services.auth.clone(),
        refresh_session: state.app_ctx.repos.refresh_session.clone(),
        config: state.app_ctx.config.clone(),
    };
    match application::auth::register_and_login::execute(
        &deps,
        application::auth::commands::RegisterAndLoginCommand {
            email: form.email,
            username: form.username,
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
        Err(_) => {
            Redirect::to("/register?error=Registration+failed.+Please+try+again.").into_response()
        }
    }
}
