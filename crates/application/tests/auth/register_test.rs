use crate::support::TestContextBuilder;
use crate::support::register_command;
use std::sync::Arc;

use domain::ports::UserRepository;
use domain::testing::InMemoryStore;
use domain::value_objects::Email;

use application::auth::{commands::RegisterCommand, deps::RegisterDeps, register};

fn cmd(email: &str) -> RegisterCommand {
    register_command(email, "alice")
}

#[tokio::test]
async fn registering_creates_the_user() {
    let users = InMemoryStore::new();
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
async fn registering_with_a_duplicate_email_fails() {
    let users = InMemoryStore::new();
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
async fn registering_with_a_short_password_fails() {
    let b = TestContextBuilder::new();
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    let result = register::execute(
        &deps,
        RegisterCommand {
            password: "short".to_string(),
            ..register_command("x@y.com", "testuser")
        },
    )
    .await;
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("8 characters"),
        "expected password length error, got: {err}"
    );
}

#[tokio::test]
async fn registration_refuses_a_retired_username() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&deps, register_command("old@example.com", "ghost"))
        .await
        .unwrap();
    let old = users
        .find_by_email(&Email::new("old@example.com".into()).unwrap())
        .await
        .unwrap()
        .unwrap();
    users.mark_deleted(old.id(), &[]).await.unwrap();
    users.delete_user(old.id());

    let err = register::execute(&deps, register_command("new@example.com", "Ghost"))
        .await
        .unwrap_err();

    assert!(
        matches!(&err, domain::errors::DomainError::ValidationError(m) if m == "Username already taken"),
        "got {err:?}"
    );
}
