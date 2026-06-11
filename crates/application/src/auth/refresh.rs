use chrono::{Duration, Utc};
use uuid::Uuid;

use domain::{errors::DomainError, models::RefreshSession};

use crate::context::AppContext;

pub struct RefreshResult {
    pub token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn execute(
    ctx: &AppContext,
    old_refresh_token: &str,
) -> Result<RefreshResult, DomainError> {
    let session = ctx
        .repos
        .refresh_session
        .get_by_token(old_refresh_token)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("Invalid refresh token".into()))?;

    if session.expires_at < Utc::now() {
        ctx.repos.refresh_session.revoke(old_refresh_token).await?;
        return Err(DomainError::Unauthorized("Refresh token expired".into()));
    }

    // Revoke old token (rotation)
    ctx.repos.refresh_session.revoke(old_refresh_token).await?;

    // Generate new access token
    let generated = ctx.services.auth.generate_token(&session.user_id).await?;

    // Create new refresh session
    let new_refresh_token = Uuid::new_v4().to_string();
    let refresh_expires = Utc::now() + Duration::seconds(ctx.config.refresh_ttl_seconds as i64);
    let new_session = RefreshSession {
        id: Uuid::new_v4(),
        user_id: session.user_id,
        token: new_refresh_token.clone(),
        expires_at: refresh_expires,
        created_at: Utc::now(),
    };
    ctx.repos.refresh_session.create(&new_session).await?;

    Ok(RefreshResult {
        token: generated.token,
        refresh_token: new_refresh_token,
        expires_at: generated.expires_at,
    })
}

#[cfg(test)]
#[path = "tests/refresh.rs"]
mod tests;
