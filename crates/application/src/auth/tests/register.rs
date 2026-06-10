use std::sync::Arc;

use domain::models::UserRole;
use domain::ports::UserRepository;
use domain::testing::InMemoryUserRepository;
use domain::value_objects::Email;

use crate::{auth::commands::RegisterCommand, auth::register, test_helpers::TestContextBuilder};

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
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    register::execute(&ctx, cmd("alice@example.com"))
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
    let ctx = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .build();

    register::execute(&ctx, cmd("bob@example.com"))
        .await
        .unwrap();
    let result = register::execute(&ctx, cmd("bob@example.com")).await;
    assert!(result.is_err(), "duplicate email should fail");
}

#[tokio::test]
async fn test_register_short_password_fails() {
    let ctx = TestContextBuilder::new().build();
    let result = register::execute(
        &ctx,
        RegisterCommand {
            email: "x@y.com".to_string(),
            username: "testuser".to_string(),
            password: "short".to_string(),
            role: UserRole::Standard,
        },
    )
    .await;
    assert!(result.is_err());
}
