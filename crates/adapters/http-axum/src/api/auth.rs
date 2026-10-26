use crate::extractors::Body;
use crate::{errors::ApiError, state::AppState};
use api_types::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
};
use application::auth::{
    commands::RegisterCommand, login as login_uc, queries::LoginCommand, register as register_uc,
};
use axum::{Json, extract::State, http::StatusCode};

#[utoipa::path(
    post, path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = api_types::ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Body(req): Body<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let result = login_uc::execute(
        &state.app_ctx.deps.auth.login,
        LoginCommand {
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
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Body(req): Body<RegisterRequest>,
) -> Result<StatusCode, ApiError> {
    register_uc::execute(
        &state.app_ctx.deps.auth.register,
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
        (status = 401, description = "Invalid or expired refresh token", body = api_types::ErrorResponse),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Body(req): Body<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let result =
        application::auth::refresh::execute(&state.app_ctx.deps.auth.refresh, &req.refresh_token)
            .await?;
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
    Body(req): Body<LogoutRequest>,
) -> StatusCode {
    let _ = application::auth::logout::execute(&state.app_ctx.deps.auth.logout, &req.refresh_token)
        .await;
    StatusCode::NO_CONTENT
}
