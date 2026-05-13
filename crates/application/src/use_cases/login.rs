use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::{errors::DomainError, value_objects::Email};

use crate::{context::AppContext, queries::LoginQuery};

pub struct LoginResult {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn execute(ctx: &AppContext, query: LoginQuery) -> Result<LoginResult, DomainError> {
    let email = Email::new(query.email)?;
    let user = ctx
        .user_repository
        .find_by_email(&email)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("Invalid credentials".into()))?;

    let valid = ctx
        .password_hasher
        .verify(&query.password, user.password_hash())
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use domain::models::UserRole;
    use domain::testing::InMemoryUserRepository;

    use crate::{
        commands::RegisterCommand,
        queries::LoginQuery,
        test_helpers::TestContextBuilder,
        use_cases::{login, register},
    };

    async fn setup_user(ctx: &crate::context::AppContext, email: &str, password: &str) {
        register::execute(
            ctx,
            RegisterCommand {
                email: email.to_string(),
                username: "testuser".to_string(),
                password: password.to_string(),
                role: UserRole::Standard,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_login_valid_credentials_returns_token() {
        let users = InMemoryUserRepository::new();
        let ctx = TestContextBuilder::new()
            .with_users(Arc::clone(&users) as _)
            .build();

        setup_user(&ctx, "carol@example.com", "secret123").await;

        let result = login::execute(
            &ctx,
            LoginQuery {
                email: "carol@example.com".into(),
                password: "secret123".into(),
            },
        )
        .await
        .unwrap();

        assert!(!result.token.is_empty());
        assert_eq!(result.email, "carol@example.com");
    }

    #[tokio::test]
    async fn test_login_wrong_password_fails() {
        let users = InMemoryUserRepository::new();
        let ctx = TestContextBuilder::new()
            .with_users(Arc::clone(&users) as _)
            .build();

        setup_user(&ctx, "dave@example.com", "correct_password").await;

        let result = login::execute(
            &ctx,
            LoginQuery {
                email: "dave@example.com".into(),
                password: "wrong_password".into(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_login_unknown_email_fails() {
        let ctx = TestContextBuilder::new().build();

        let result = login::execute(
            &ctx,
            LoginQuery {
                email: "nobody@example.com".into(),
                password: "anything".into(),
            },
        )
        .await;

        assert!(result.is_err());
    }
}
