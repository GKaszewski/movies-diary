use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::UserRepository,
    testing::InMemoryStore,
    value_objects::{Email, UserId},
};

use application::auth::{
    authenticate,
    deps::{AuthenticateDeps, RegisterDeps},
    register,
};

use crate::support::{TestContextBuilder, register_command};

#[tokio::test]
async fn a_live_users_token_authenticates() {
    let store = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&store) as _);
    let reg = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&reg, register_command("alice@example.com", "alice"))
        .await
        .unwrap();
    let id = store
        .find_by_email(&Email::new("alice@example.com".into()).unwrap())
        .await
        .unwrap()
        .unwrap()
        .id()
        .clone();
    let deps = AuthenticateDeps {
        auth: b.auth_service.clone(),
        user: b.user_repo.clone(),
    };

    let authenticated = authenticate::execute(&deps, &id.value().to_string())
        .await
        .unwrap();

    assert_eq!(authenticated, id);
}

#[tokio::test]
async fn a_deleted_users_token_is_rejected() {
    let store = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&store) as _);
    let reg = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&reg, register_command("alice@example.com", "alice"))
        .await
        .unwrap();
    let id = store
        .find_by_email(&Email::new("alice@example.com".into()).unwrap())
        .await
        .unwrap()
        .unwrap()
        .id()
        .clone();
    store.mark_deleted(&id, &[]).await.unwrap();
    let deps = AuthenticateDeps {
        auth: b.auth_service.clone(),
        user: b.user_repo.clone(),
    };

    let err = authenticate::execute(&deps, &id.value().to_string())
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}

#[tokio::test]
async fn an_unknown_users_token_is_rejected() {
    let b = TestContextBuilder::new();
    let deps = AuthenticateDeps {
        auth: b.auth_service.clone(),
        user: b.user_repo.clone(),
    };

    let err = authenticate::execute(&deps, &UserId::generate().value().to_string())
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}

#[tokio::test]
async fn an_invalid_token_is_rejected() {
    let b = TestContextBuilder::new();
    let deps = AuthenticateDeps {
        auth: b.auth_service.clone(),
        user: b.user_repo.clone(),
    };

    let err = authenticate::execute(&deps, "not-a-token")
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}
