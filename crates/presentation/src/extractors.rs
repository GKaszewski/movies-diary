use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Redirect},
};
use domain::{errors::DomainError, value_objects::UserId};

use crate::{errors::ApiError, state::AppState};

pub struct AuthenticatedUser(pub UserId);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                ApiError(DomainError::Unauthorized(
                    "Missing or invalid auth token".into(),
                ))
            })?;
        let user_id = app_state
            .app_ctx
            .services
            .auth
            .validate_token(token)
            .await?;
        Ok(AuthenticatedUser(user_id))
    }
}

pub struct OptionalCookieUser(pub Option<UserId>);
pub struct RequiredCookieUser(pub UserId);

fn extract_token_from_cookie(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|c| c.trim().strip_prefix("token=").map(str::to_string))
        })
}

impl<S> FromRequestParts<S> for OptionalCookieUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let Some(token) = extract_token_from_cookie(parts) else {
            return Ok(OptionalCookieUser(None));
        };
        let user_id = app_state
            .app_ctx
            .services
            .auth
            .validate_token(&token)
            .await
            .ok();
        Ok(OptionalCookieUser(user_id))
    }
}

impl<S> FromRequestParts<S> for RequiredCookieUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_token_from_cookie(parts)
            .ok_or_else(|| Redirect::to("/login").into_response())?;
        let user_id = app_state
            .app_ctx
            .services
            .auth
            .validate_token(&token)
            .await
            .map_err(|_| Redirect::to("/login").into_response())?;
        Ok(RequiredCookieUser(user_id))
    }
}

pub struct AdminApiUser(pub UserId);

impl<S> FromRequestParts<S> for AdminApiUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let app_state = AppState::from_ref(state);
        let user = app_state
            .app_ctx
            .repos
            .user
            .find_by_id(&user_id)
            .await
            .map_err(ApiError)?
            .ok_or_else(|| ApiError(DomainError::NotFound("user not found".into())))?;
        match user.role() {
            domain::models::UserRole::Admin => Ok(AdminApiUser(user_id)),
            _ => Err(ApiError(DomainError::Forbidden("admin only".into()))),
        }
    }
}

pub struct AdminUser(pub UserId);

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let RequiredCookieUser(user_id) =
            RequiredCookieUser::from_request_parts(parts, state).await?;
        let user = app_state
            .app_ctx
            .repos
            .user
            .find_by_id(&user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())?;
        match user.role() {
            domain::models::UserRole::Admin => Ok(AdminUser(user_id)),
            _ => Err(StatusCode::FORBIDDEN.into_response()),
        }
    }
}

#[cfg(test)]
#[path = "tests/extractors.rs"]
pub(crate) mod tests;
