use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::WatchlistEntry;
use domain::ports::WatchlistRepository;
use domain::testing::{InMemoryStore, RefusingWatchlistRepository};
use domain::value_objects::{MovieId, UserId};
use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::watchlist::{
    commands::RemoveFromWatchlistCommand, deps::RemoveFromWatchlistDeps, remove,
};

#[tokio::test]
async fn removing_an_entry_records_the_removal_and_its_event_together() {
    let watchlist = InMemoryStore::new();
    let uid = Uuid::new_v4();
    let mid = Uuid::new_v4();
    watchlist
        .add(
            &WatchlistEntry::new(UserId::from_uuid(uid), MovieId::from_uuid(mid)),
            &[],
        )
        .await
        .unwrap();

    let deps = RemoveFromWatchlistDeps {
        watchlist: Arc::clone(&watchlist) as _,
    };
    remove::execute(
        &deps,
        RemoveFromWatchlistCommand {
            user_id: uid,
            movie_id: mid,
        },
    )
    .await
    .unwrap();

    assert_eq!(watchlist.watchlist_count(), 0);
    let recorded = watchlist.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        DomainEvent::WatchlistEntryRemoved { .. }
    ));
}

#[tokio::test]
async fn fails_when_not_on_watchlist() {
    let b = TestContextBuilder::new();
    let deps = RemoveFromWatchlistDeps {
        watchlist: b.watchlist_repo.clone(),
    };
    let result = remove::execute(
        &deps,
        RemoveFromWatchlistCommand {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn a_failed_removal_records_no_event() {
    let store = InMemoryStore::new();
    let deps = RemoveFromWatchlistDeps {
        watchlist: Arc::clone(&store) as _,
    };

    let result = remove::execute(
        &deps,
        RemoveFromWatchlistCommand {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn a_refused_remove_propagates_the_refusal() {
    let deps = RemoveFromWatchlistDeps {
        watchlist: Arc::new(RefusingWatchlistRepository),
    };

    let result = remove::execute(
        &deps,
        RemoveFromWatchlistCommand {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(m)) if m == "refused"
    ));
}
