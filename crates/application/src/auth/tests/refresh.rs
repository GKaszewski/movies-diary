use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::commands::RegisterCommand,
    auth::queries::LoginQuery,
    auth::{login, refresh, register},
    test_helpers::TestContextBuilder,
};

async fn login_user(ctx: &crate::context::AppContext) -> login::LoginResult {
    register::execute(
        ctx,
        RegisterCommand {
            email: "alice@example.com".to_string(),
            username: "alice".to_string(),
            password: "password123".to_string(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    login::execute(
        ctx,
        LoginQuery {
            email: "alice@example.com".into(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn refresh_returns_new_tokens() {
    let users = InMemoryUserRepository::new();
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    let login_result = login_user(&ctx).await;

    let result = refresh::execute(&ctx, &login_result.refresh_token)
        .await
        .unwrap();

    assert!(!result.token.is_empty());
    assert!(!result.refresh_token.is_empty());
    assert_ne!(result.refresh_token, login_result.refresh_token);
}

#[tokio::test]
async fn refresh_rotates_token_old_one_invalid() {
    let users = InMemoryUserRepository::new();
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    let login_result = login_user(&ctx).await;
    let old_token = login_result.refresh_token.clone();

    refresh::execute(&ctx, &old_token).await.unwrap();

    let retry = refresh::execute(&ctx, &old_token).await;
    assert!(retry.is_err());
}

#[tokio::test]
async fn refresh_with_new_token_works() {
    let users = InMemoryUserRepository::new();
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    let login_result = login_user(&ctx).await;

    let first = refresh::execute(&ctx, &login_result.refresh_token)
        .await
        .unwrap();

    let second = refresh::execute(&ctx, &first.refresh_token).await.unwrap();

    assert!(!second.token.is_empty());
    assert_ne!(second.refresh_token, first.refresh_token);
}

#[tokio::test]
async fn refresh_with_unknown_token_fails() {
    let ctx = TestContextBuilder::new().build();

    let result = refresh::execute(&ctx, "nonexistent-token").await;
    assert!(result.is_err());
}
