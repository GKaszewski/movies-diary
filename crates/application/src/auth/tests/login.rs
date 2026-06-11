use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::commands::RegisterCommand,
    auth::queries::LoginQuery,
    auth::{login, register},
    test_helpers::TestContextBuilder,
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
    assert!(!result.refresh_token.is_empty());
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
