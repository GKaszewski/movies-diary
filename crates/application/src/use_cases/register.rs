use domain::{errors::DomainError, models::User, value_objects::Email};

use crate::{commands::RegisterCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: RegisterCommand) -> Result<(), DomainError> {
    if !ctx.config.allow_registration {
        return Err(DomainError::Unauthorized("Registration is disabled".into()));
    }

    let email = Email::new(cmd.email)?;

    if ctx.user_repository.find_by_email(&email).await?.is_some() {
        return Err(DomainError::ValidationError("Email already registered".into()));
    }

    let hash = ctx.password_hasher.hash(&cmd.password).await?;
    ctx.user_repository.save(&User::new(email, hash)).await
}
