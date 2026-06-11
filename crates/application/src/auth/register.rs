use domain::{
    errors::DomainError,
    models::User,
    value_objects::{Email, Password, Username},
};

use crate::auth::{commands::RegisterCommand, deps::RegisterDeps};

pub async fn execute(deps: &RegisterDeps, cmd: RegisterCommand) -> Result<(), DomainError> {
    if !deps.config.allow_registration {
        return Err(DomainError::Unauthorized("Registration is disabled".into()));
    }

    let password = Password::new(cmd.password)?;
    let email = Email::new(cmd.email)?;
    let username = Username::new(cmd.username)?;

    if deps.user.find_by_email(&email).await?.is_some() {
        return Err(DomainError::ValidationError(
            "Email already registered".into(),
        ));
    }

    if deps.user.find_by_username(&username).await?.is_some() {
        return Err(DomainError::ValidationError(
            "Username already taken".into(),
        ));
    }

    let hash = deps.password_hasher.hash(password.value()).await?;
    deps.user
        .save(&User::new(email, username, hash, cmd.role))
        .await
}

#[cfg(test)]
#[path = "tests/register.rs"]
mod tests;
