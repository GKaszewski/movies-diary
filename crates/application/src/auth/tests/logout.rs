use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::{
        commands::RegisterCommand,
        deps::{LoginDeps, RefreshDeps, RegisterDeps},
        login, logout, refresh, register,
        queries::LoginQuery,
    },
    test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);

    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &reg_deps,
        RegisterCommand {
            email: "bob@example.com".to_string(),
            username: "bob".to_string(),
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
    let login_result = login::execute(
        &login_deps,
        LoginQuery {
            email: "bob@example.com".into(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap();

    logout::execute(
        b.refresh_session_repo.clone(),
        &login_result.refresh_token,
    )
    .await
    .unwrap();

    let refresh_deps = RefreshDeps {
        refresh_session: b.refresh_session_repo.clone(),
        auth: b.auth_service.clone(),
        config: b.config.clone(),
    };
    let refresh_attempt = refresh::execute(&refresh_deps, &login_result.refresh_token).await;
    assert!(refresh_attempt.is_err());
}

#[tokio::test]
async fn logout_with_unknown_token_succeeds() {
    let b = TestContextBuilder::new();
    let result = logout::execute(b.refresh_session_repo.clone(), "nonexistent-token").await;
    assert!(result.is_ok());
}
