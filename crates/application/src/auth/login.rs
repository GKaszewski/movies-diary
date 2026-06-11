use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use domain::{errors::DomainError, models::RefreshSession, value_objects::Email};

use crate::{auth::queries::LoginQuery, context::AppContext};

pub struct LoginResult {
    pub token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub role: String,
}

pub async fn execute(ctx: &AppContext, query: LoginQuery) -> Result<LoginResult, DomainError> {
    let email = Email::new(query.email)?;
    let user = ctx
        .repos
        .user
        .find_by_email(&email)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("Invalid credentials".into()))?;

    let valid = ctx
        .services
        .password_hasher
        .verify(&query.password, user.password_hash())
        .await?;
    if !valid {
        return Err(DomainError::Unauthorized("Invalid credentials".into()));
    }

    let generated = ctx.services.auth.generate_token(user.id()).await?;

    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expires = Utc::now() + Duration::seconds(ctx.config.refresh_ttl_seconds as i64);
    let session = RefreshSession {
        id: Uuid::new_v4(),
        user_id: user.id().clone(),
        token: refresh_token.clone(),
        expires_at: refresh_expires,
        created_at: Utc::now(),
    };
    ctx.repos.refresh_session.create(&session).await?;

    Ok(LoginResult {
        token: generated.token,
        refresh_token,
        user_id: user.id().value(),
        email: user.email().value().to_string(),
        expires_at: generated.expires_at,
        role: user.role().as_str().into(),
    })
}

#[cfg(test)]
#[path = "tests/login.rs"]
mod tests;
