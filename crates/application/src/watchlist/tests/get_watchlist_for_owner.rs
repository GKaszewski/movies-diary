use std::sync::Arc;

use uuid::Uuid;

use domain::models::UserRole;
use domain::testing::{
    InMemoryRemoteWatchlistRepository, InMemoryUserRepository, InMemoryWatchlistRepository,
    PanicRemoteWatchlistRepository, PanicWatchlistRepository,
};
use domain::value_objects::Email;

use crate::auth::commands::RegisterCommand;
use crate::auth::deps::RegisterDeps;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::watchlist::deps::GetWatchlistForOwnerDeps;
use crate::watchlist::get_watchlist_for_owner::{self, WatchlistView};

async fn setup_user(b: &TestContextBuilder, email: &str, username: &str) {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &deps,
        RegisterCommand {
            email: email.into(),
            username: username.into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
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

    // If the local-row decision were wrong and this reached the remote arm, the
    // panic port below would fail the test instead of silently passing.
    let deps = GetWatchlistForOwnerDeps {
        user: b.user_repo.clone(),
        watchlist: InMemoryWatchlistRepository::new(),
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

    let remote = InMemoryRemoteWatchlistRepository::with_entries(vec![
        domain::models::RemoteWatchlistEntry {
            ap_id: "https://remote.example/ap/1".into(),
            actor_url: "https://remote.example/users/owner".into(),
            movie_title: "Remote Movie".into(),
            release_year: 2021,
            external_metadata_id: None,
            poster_url: None,
            added_at: chrono::Utc::now(),
        },
    ]);

    // No user row exists for `owner_id` in this fresh, empty user repo — the
    // local-vs-remote decision must fall through to the remote arm. The panic
    // port below proves the local arm is never touched.
    let deps = GetWatchlistForOwnerDeps {
        user: InMemoryUserRepository::new(),
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

/// Pins a pre-existing quirk: the old handler validated `limit`/`offset` only on
/// the local branch (inside `watchlist::get::execute`'s `PageParams::new` call) —
/// the remote branch never looked at them at all. An out-of-range limit must
/// still surface as a validation error on the local arm.
#[tokio::test]
async fn watchlist_for_owner_local_arm_validates_limit() {
    let b = TestContextBuilder::new();
    setup_user(&b, "badlimit@test.com", "badlimituser").await;

    let email = Email::new("badlimit@test.com".into()).unwrap();
    let user = b.user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let deps = GetWatchlistForOwnerDeps {
        user: b.user_repo.clone(),
        watchlist: InMemoryWatchlistRepository::new(),
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

/// Same quirk, other direction: an out-of-range limit is silently ignored on the
/// remote arm, exactly as before — it must not turn into a validation error just
/// because the use case now owns the branch decision.
#[tokio::test]
async fn watchlist_for_owner_remote_arm_ignores_invalid_limit() {
    let owner_id = Uuid::new_v4();
    let remote = InMemoryRemoteWatchlistRepository::new();

    let deps = GetWatchlistForOwnerDeps {
        user: InMemoryUserRepository::new(),
        watchlist: Arc::new(PanicWatchlistRepository),
        remote_watchlist: remote,
    };

    let view = get_watchlist_for_owner::execute(&deps, owner_id, Some(0), Some(0))
        .await
        .unwrap();
    assert!(matches!(view, WatchlistView::Remote(_)), "expected Remote");
}
