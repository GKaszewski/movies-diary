use crate::support::TestContextBuilder;
use crate::support::{login_command, register_command};
use std::sync::Arc;

use domain::testing::InMemoryStore;

use application::auth::{
    deps::{LoginDeps, LogoutDeps, RefreshDeps, RegisterDeps},
    login, logout, refresh, register,
};

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);

    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&reg_deps, register_command("bob@example.com", "bob"))
        .await
        .unwrap();

    let login_deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let login_result = login::execute(&login_deps, login_command("bob@example.com", "password123"))
        .await
        .unwrap();

    let logout_deps = LogoutDeps {
        refresh_session: b.refresh_session_repo.clone(),
    };
    logout::execute(&logout_deps, &login_result.refresh_token)
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
    let logout_deps = LogoutDeps {
        refresh_session: b.refresh_session_repo.clone(),
    };
    let result = logout::execute(&logout_deps, "nonexistent-token").await;
    assert!(result.is_ok());
}
