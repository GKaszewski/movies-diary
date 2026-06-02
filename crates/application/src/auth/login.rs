use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::{errors::DomainError, value_objects::Email};

use crate::{auth::queries::LoginQuery, context::AppContext};

pub struct LoginResult {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
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

    Ok(LoginResult {
        token: generated.token,
        user_id: user.id().value(),
        email: user.email().value().to_string(),
        expires_at: generated.expires_at,
    })
}

#[cfg(test)]
#[path = "tests/login.rs"]
mod tests;
