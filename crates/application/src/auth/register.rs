use domain::{
    errors::DomainError,
    models::User,
    value_objects::{Email, Username},
};

use crate::{auth::commands::RegisterCommand, context::AppContext};

const MIN_PASSWORD_LENGTH: usize = 8;

pub async fn execute(ctx: &AppContext, cmd: RegisterCommand) -> Result<(), DomainError> {
    if !ctx.config.allow_registration {
        return Err(DomainError::Unauthorized("Registration is disabled".into()));
    }

    if cmd.password.len() < MIN_PASSWORD_LENGTH {
        return Err(DomainError::ValidationError(
            "Password must be at least 8 characters".into(),
        ));
    }

    let email = Email::new(cmd.email)?;
    let username = Username::new(cmd.username)?;

    if ctx.repos.user.find_by_email(&email).await?.is_some() {
        return Err(DomainError::ValidationError(
            "Email already registered".into(),
        ));
    }

    if ctx.repos.user.find_by_username(&username).await?.is_some() {
        return Err(DomainError::ValidationError(
            "Username already taken".into(),
        ));
    }

    let hash = ctx.services.password_hasher.hash(&cmd.password).await?;
    ctx.repos
        .user
        .save(&User::new(email, username, hash, cmd.role))
        .await
}

#[cfg(test)]
#[path = "tests/register.rs"]
mod tests;
