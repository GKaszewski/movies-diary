use std::sync::Arc;

use domain::models::UserRole;
use domain::ports::UserRepository;
use domain::testing::InMemoryUserRepository;
use domain::value_objects::Email;

use crate::{
    auth::{commands::RegisterCommand, deps::RegisterDeps, register},
    test_helpers::TestContextBuilder,
};

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
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };

    register::execute(&deps, cmd("alice@example.com"))
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
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };

    register::execute(&deps, cmd("bob@example.com"))
        .await
        .unwrap();
    let result = register::execute(&deps, cmd("bob@example.com")).await;
    assert!(result.is_err(), "duplicate email should fail");
}

#[tokio::test]
async fn test_register_short_password_fails() {
    let b = TestContextBuilder::new();
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    let result = register::execute(
        &deps,
        RegisterCommand {
            email: "x@y.com".to_string(),
            username: "testuser".to_string(),
            password: "short".to_string(),
            role: UserRole::Standard,
        },
    )
    .await;
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("8 characters"),
        "expected password length error, got: {err}"
    );
}
