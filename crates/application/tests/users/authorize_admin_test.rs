use crate::support::TestContextBuilder;
use crate::support::register_command;
use std::sync::Arc;

use domain::models::UserRole;
use domain::testing::InMemoryStore;

use application::{
    auth::{commands::RegisterCommand, deps::RegisterDeps, register},
    users::{authorize_admin, deps::AuthorizeAdminDeps},
};

async fn register_user(
    b: &TestContextBuilder,
    email: &str,
    username: &str,
    role: UserRole,
) -> domain::models::User {
    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &reg_deps,
        RegisterCommand {
            role,
            ..register_command(email, username)
        },
    )
    .await
    .unwrap();

    b.user_repo
        .find_by_email(&domain::value_objects::Email::new(email.into()).unwrap())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn authorize_admin_is_true_for_an_admin_user() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let user = register_user(&b, "admin@example.com", "admin_user", UserRole::Admin).await;

    let deps = AuthorizeAdminDeps {
        user: b.user_repo.clone(),
    };
    let result = authorize_admin::execute(&deps, user.id().value())
        .await
        .unwrap();

    assert_eq!(result, Some(true));
}

#[tokio::test]
async fn authorize_admin_is_false_for_a_regular_user() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let user = register_user(
        &b,
        "standard@example.com",
        "standard_user",
        UserRole::Standard,
    )
    .await;

    let deps = AuthorizeAdminDeps {
        user: b.user_repo.clone(),
    };
    let result = authorize_admin::execute(&deps, user.id().value())
        .await
        .unwrap();

    assert_eq!(result, Some(false));
}

#[tokio::test]
async fn authorize_admin_is_not_found_for_unknown_user() {
    let b = TestContextBuilder::new();

    let deps = AuthorizeAdminDeps {
        user: b.user_repo.clone(),
    };
    let result = authorize_admin::execute(&deps, uuid::Uuid::new_v4())
        .await
        .unwrap();

    assert_eq!(result, None);
}
