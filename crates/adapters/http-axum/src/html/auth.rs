use crate::html::templates::{LoginTemplate, RegisterTemplate};
use crate::{
    html::csrf::{CsrfToken, secure_flag},
    html::forms::{ErrorQuery, LoginForm, RegisterForm},
    html::render::render_page,
    state::AppState,
};
use api_types::HtmlPageContext;
use application::auth::{login as login_uc, queries::LoginCommand};
use axum::{
    Form,
    extract::{Extension, Query, State},
    http::{HeaderValue, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
};
use chrono::Utc;

fn set_cookie_header(
    token: &str,
    max_age: i64,
    secure_cookies: bool,
) -> (axum::http::HeaderName, HeaderValue) {
    let val = format!(
        "token={}; HttpOnly; Path=/; SameSite=Strict; Max-Age={}{}",
        token,
        max_age,
        secure_flag(secure_cookies)
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
        pending_follow_count: 0,
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match login_uc::execute(
        &state.app_ctx.deps.auth.login,
        LoginCommand {
            email: form.email,
            password: form.password,
        },
    )
    .await
    {
        Ok(result) => {
            let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
            let cookie =
                set_cookie_header(&result.token, max_age, state.app_ctx.config.secure_cookies);
            ([cookie], Redirect::to("/")).into_response()
        }
        Err(_) => Redirect::to("/login?error=Invalid+credentials").into_response(),
    }
}

pub(super) async fn get_logout(State(state): State<AppState>) -> impl IntoResponse {
    let val = format!(
        "token=; HttpOnly; Path=/; SameSite=Strict; Max-Age=0{}",
        secure_flag(state.app_ctx.config.secure_cookies)
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
        pending_follow_count: 0,
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match application::auth::register_and_login::execute(
        &state.app_ctx.deps.auth.register_and_login,
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
            let cookie =
                set_cookie_header(&result.token, max_age, state.app_ctx.config.secure_cookies);
            ([cookie], Redirect::to("/")).into_response()
        }
        Err(_) => {
            Redirect::to("/register?error=Registration+failed.+Please+try+again.").into_response()
        }
    }
}
