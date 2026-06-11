use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::commands::RegisterCommand,
    auth::queries::LoginQuery,
    auth::{login, logout, refresh, register},
    test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let users = InMemoryUserRepository::new();
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "bob@example.com".to_string(),
            username: "bob".to_string(),
            password: "password123".to_string(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let login_result = login::execute(
        &ctx,
        LoginQuery {
            email: "bob@example.com".into(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap();

    logout::execute(&ctx, &login_result.refresh_token)
        .await
        .unwrap();

    let refresh_attempt = refresh::execute(&ctx, &login_result.refresh_token).await;
    assert!(refresh_attempt.is_err());
}

#[tokio::test]
async fn logout_with_unknown_token_succeeds() {
    let ctx = TestContextBuilder::new().build();
    let result = logout::execute(&ctx, "nonexistent-token").await;
    assert!(result.is_ok());
}
