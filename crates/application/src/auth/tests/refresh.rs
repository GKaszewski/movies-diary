use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::{
        commands::RegisterCommand,
        deps::{LoginDeps, RefreshDeps, RegisterDeps},
        login,
        queries::LoginQuery,
        refresh, register,
    },
    test_helpers::TestContextBuilder,
};

async fn login_user(b: &TestContextBuilder) -> login::LoginResult {
    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &reg_deps,
        RegisterCommand {
            email: "alice@example.com".to_string(),
            username: "alice".to_string(),
            password: "password123".to_string(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let login_deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    login::execute(
        &login_deps,
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
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let login_result = login_user(&b).await;

    let deps = RefreshDeps {
        refresh_session: b.refresh_session_repo.clone(),
        auth: b.auth_service.clone(),
        config: b.config.clone(),
    };
    let result = refresh::execute(&deps, &login_result.refresh_token)
        .await
        .unwrap();

    assert!(!result.token.is_empty());
    assert!(!result.refresh_token.is_empty());
    assert_ne!(result.refresh_token, login_result.refresh_token);
}

#[tokio::test]
async fn refresh_rotates_token_old_one_invalid() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let login_result = login_user(&b).await;
    let old_token = login_result.refresh_token.clone();

    let deps = RefreshDeps {
        refresh_session: b.refresh_session_repo.clone(),
        auth: b.auth_service.clone(),
        config: b.config.clone(),
    };
    refresh::execute(&deps, &old_token).await.unwrap();

    let retry = refresh::execute(&deps, &old_token).await;
    assert!(retry.is_err());
}

#[tokio::test]
async fn refresh_with_new_token_works() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let login_result = login_user(&b).await;

    let deps = RefreshDeps {
        refresh_session: b.refresh_session_repo.clone(),
        auth: b.auth_service.clone(),
        config: b.config.clone(),
    };
    let first = refresh::execute(&deps, &login_result.refresh_token)
        .await
        .unwrap();

    let second = refresh::execute(&deps, &first.refresh_token).await.unwrap();

    assert!(!second.token.is_empty());
    assert_ne!(second.refresh_token, first.refresh_token);
}

#[tokio::test]
async fn refresh_with_unknown_token_fails() {
    let b = TestContextBuilder::new();
    let deps = RefreshDeps {
        refresh_session: b.refresh_session_repo.clone(),
        auth: b.auth_service.clone(),
        config: b.config.clone(),
    };
    let result = refresh::execute(&deps, "nonexistent-token").await;
    assert!(result.is_err());
}
