use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::UserRepository,
    testing::InMemoryStore,
    value_objects::{Email, UserId},
};

use application::{
    auth::{deps::RegisterDeps, register},
    users::{deps::EnsureActiveDeps, ensure_active},
};

use crate::support::{TestContextBuilder, register_command};

#[tokio::test]
async fn a_live_user_is_active() {
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
    let deps = EnsureActiveDeps {
        user: b.user_repo.clone(),
    };

    ensure_active::execute(&deps, &id).await.unwrap();
}

#[tokio::test]
async fn a_deleted_user_is_rejected() {
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
    let deps = EnsureActiveDeps {
        user: b.user_repo.clone(),
    };

    let err = ensure_active::execute(&deps, &id).await.unwrap_err();

    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}

#[tokio::test]
async fn an_unknown_user_is_rejected() {
    let b = TestContextBuilder::new();
    let deps = EnsureActiveDeps {
        user: b.user_repo.clone(),
    };

    let err = ensure_active::execute(&deps, &UserId::generate())
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}
