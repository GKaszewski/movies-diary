use crate::support::register_command;
use std::sync::Arc;

use uuid::Uuid;

use domain::testing::{InMemoryStore, PanicRemoteWatchlistRepository, PanicWatchlistRepository};
use domain::value_objects::Email;

use crate::support::TestContextBuilder;
use application::auth::deps::RegisterDeps;
use application::auth::register;
use application::watchlist::deps::GetWatchlistForOwnerDeps;
use application::watchlist::get_watchlist_for_owner::{self, WatchlistView};

async fn setup_user(b: &TestContextBuilder, email: &str, username: &str) {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&deps, register_command(email, username))
        .await
        .unwrap();
}

#[tokio::test]
async fn watchlist_for_owner_returns_local_when_user_row_exists() {
    let b = TestContextBuilder::new();
    setup_user(&b, "owner@test.com", "owneruser").await;

    let email = Email::new("owner@test.com".into()).unwrap();
    let user = b.user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let deps = GetWatchlistForOwnerDeps {
        user: b.user_repo.clone(),
        watchlist: InMemoryStore::new(),
        remote_watchlist: Arc::new(PanicRemoteWatchlistRepository),
    };

    let view = get_watchlist_for_owner::execute(&deps, uid, Some(20), Some(0))
        .await
        .unwrap();

    assert!(matches!(view, WatchlistView::Local(_)), "expected Local");
}

#[tokio::test]
async fn watchlist_for_owner_returns_remote_when_no_local_row() {
    let owner_id = Uuid::new_v4();

    let remote = InMemoryStore::with_remote_watchlist(vec![domain::models::RemoteWatchlistEntry {
        ap_id: "https://remote.example/ap/1".into(),
        actor_url: "https://remote.example/users/owner".into(),
        movie_title: "Remote Movie".into(),
        release_year: 2021,
        external_metadata_id: None,
        poster_url: None,
        added_at: chrono::Utc::now(),
    }]);

    let deps = GetWatchlistForOwnerDeps {
        user: InMemoryStore::new(),
        watchlist: Arc::new(PanicWatchlistRepository),
        remote_watchlist: remote,
    };

    let view = get_watchlist_for_owner::execute(&deps, owner_id, Some(20), Some(0))
        .await
        .unwrap();

    match view {
        WatchlistView::Remote(entries) => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].movie_title, "Remote Movie");
        }
        WatchlistView::Local(_) => panic!("expected Remote"),
    }
}

#[tokio::test]
async fn watchlist_for_owner_local_arm_validates_limit() {
    let b = TestContextBuilder::new();
    setup_user(&b, "badlimit@test.com", "badlimituser").await;

    let email = Email::new("badlimit@test.com".into()).unwrap();
    let user = b.user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let deps = GetWatchlistForOwnerDeps {
        user: b.user_repo.clone(),
        watchlist: InMemoryStore::new(),
        remote_watchlist: Arc::new(PanicRemoteWatchlistRepository),
    };

    let err = match get_watchlist_for_owner::execute(&deps, uid, Some(0), Some(0)).await {
        Err(e) => e,
        Ok(_) => panic!("expected Err(ValidationError) for limit=0 on the local arm, got Ok"),
    };
    assert!(matches!(
        err,
        domain::errors::DomainError::ValidationError(_)
    ));
}

#[tokio::test]
async fn watchlist_for_owner_remote_arm_ignores_invalid_limit() {
    let owner_id = Uuid::new_v4();
    let remote = InMemoryStore::new();

    let deps = GetWatchlistForOwnerDeps {
        user: InMemoryStore::new(),
        watchlist: Arc::new(PanicWatchlistRepository),
        remote_watchlist: remote,
    };

    let view = get_watchlist_for_owner::execute(&deps, owner_id, Some(0), Some(0))
        .await
        .unwrap();
    assert!(matches!(view, WatchlistView::Remote(_)), "expected Remote");
}
