use domain::{
    errors::DomainError,
    models::User,
    value_objects::{Email, Username},
};

use crate::{commands::RegisterCommand, context::AppContext};

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

    if ctx.user_repository.find_by_email(&email).await?.is_some() {
        return Err(DomainError::ValidationError(
            "Email already registered".into(),
        ));
    }

    if ctx
        .user_repository
        .find_by_username(&username)
        .await?
        .is_some()
    {
        return Err(DomainError::ValidationError(
            "Username already taken".into(),
        ));
    }

    let hash = ctx.password_hasher.hash(&cmd.password).await?;
    ctx.user_repository
        .save(&User::new(email, username, hash, cmd.role))
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use domain::models::UserRole;
    use domain::ports::UserRepository;
    use domain::testing::InMemoryUserRepository;
    use domain::value_objects::Email;

    use crate::{commands::RegisterCommand, test_helpers::TestContextBuilder, use_cases::register};

    fn cmd(email: &str) -> RegisterCommand {
        RegisterCommand {
            email: email.to_string(),
            username: "alice".to_string(),
            password: "password123".to_string(),
            role: UserRole::Standard,
        }
    }

    #[tokio::test]
    async fn test_register_creates_user() {
        let users = InMemoryUserRepository::new();
        let ctx = TestContextBuilder::new()
            .with_users(Arc::clone(&users) as _)
            .build();

        register::execute(&ctx, cmd("alice@example.com"))
            .await
            .unwrap();

        let email = Email::new("alice@example.com".into()).unwrap();
        let user = users.find_by_email(&email).await.unwrap().unwrap();
        assert_eq!(user.email().value(), "alice@example.com");
        assert!(user.password_hash().value().starts_with("hashed:"));
    }

    #[tokio::test]
    async fn test_register_duplicate_email_fails() {
        let users = InMemoryUserRepository::new();
        let ctx = TestContextBuilder::new()
            .with_users(Arc::clone(&users) as _)
            .build();

        register::execute(&ctx, cmd("bob@example.com"))
            .await
            .unwrap();
        let result = register::execute(&ctx, cmd("bob@example.com")).await;
        assert!(result.is_err(), "duplicate email should fail");
    }
}
