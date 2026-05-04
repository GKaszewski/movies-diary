use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::{errors::DomainError, value_objects::Email};

use crate::{commands::LoginCommand, context::AppContext};

pub struct LoginResult {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn execute(ctx: &AppContext, cmd: LoginCommand) -> Result<LoginResult, DomainError> {
    let email = Email::new(cmd.email)?;
    let user = ctx
        .user_repository
        .find_by_email(&email)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("Invalid credentials".into()))?;

    let valid = ctx
        .password_hasher
        .verify(&cmd.password, user.password_hash())
        .await?;
    if !valid {
        return Err(DomainError::Unauthorized("Invalid credentials".into()));
    }

    let generated = ctx.auth_service.generate_token(user.id()).await?;

    Ok(LoginResult {
        token: generated.token,
        user_id: user.id().value(),
        email: user.email().value().to_string(),
        expires_at: generated.expires_at,
    })
}
