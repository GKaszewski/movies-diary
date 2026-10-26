use crate::support::TestContextBuilder;
use crate::support::{login_command, register_command};
use std::sync::Arc;

use domain::testing::InMemoryStore;

use application::auth::{
    commands::RegisterCommand,
    deps::{LoginDeps, RegisterDeps},
    login, register,
};

async fn setup_user(b: &TestContextBuilder, email: &str, password: &str) {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &deps,
        RegisterCommand {
            password: password.to_string(),
            ..register_command(email, "testuser")
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn login_with_valid_credentials_returns_a_token() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    setup_user(&b, "carol@example.com", "secret123").await;

    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(&deps, login_command("carol@example.com", "secret123"))
        .await
        .unwrap();

    assert!(!result.token.is_empty());
    assert!(!result.refresh_token.is_empty());
    assert_eq!(result.email, "carol@example.com");
}

#[tokio::test]
async fn login_with_the_wrong_password_fails() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    setup_user(&b, "dave@example.com", "correct_password").await;

    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(&deps, login_command("dave@example.com", "wrong_password")).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn login_with_an_unknown_email_fails() {
    let b = TestContextBuilder::new();
    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(&deps, login_command("nobody@example.com", "anything")).await;

    assert!(result.is_err());
}
