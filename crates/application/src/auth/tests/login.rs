use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryUserRepository;

use crate::{
    auth::{
        commands::RegisterCommand,
        deps::{LoginDeps, RegisterDeps},
        login,
        queries::LoginCommand,
        register,
    },
    test_helpers::TestContextBuilder,
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
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    setup_user(&b, "carol@example.com", "secret123").await;

    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(
        &deps,
        LoginCommand {
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
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    setup_user(&b, "dave@example.com", "correct_password").await;

    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(
        &deps,
        LoginCommand {
            email: "dave@example.com".into(),
            password: "wrong_password".into(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_login_unknown_email_fails() {
    let b = TestContextBuilder::new();
    let deps = LoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };
    let result = login::execute(
        &deps,
        LoginCommand {
            email: "nobody@example.com".into(),
            password: "anything".into(),
        },
    )
    .await;

    assert!(result.is_err());
}
