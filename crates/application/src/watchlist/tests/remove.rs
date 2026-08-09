use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::WatchlistEntry;
use domain::ports::WatchlistRepository;
use domain::testing::{InMemoryWatchlistRepository, NoopEventPublisher};
use domain::value_objects::{MovieId, UserId};
use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::watchlist::{
    commands::RemoveFromWatchlistCommand, deps::RemoveFromWatchlistDeps, remove,
};

#[tokio::test]
async fn removes_entry_and_emits_event() {
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let uid = Uuid::new_v4();
    let mid = Uuid::new_v4();
    watchlist
        .add(&WatchlistEntry::new(
            UserId::from_uuid(uid),
            MovieId::from_uuid(mid),
        ))
        .await
        .unwrap();

    let deps = RemoveFromWatchlistDeps {
        watchlist: Arc::clone(&watchlist) as _,
        event_publisher: Arc::clone(&events) as _,
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

    assert_eq!(watchlist.count(), 0);
    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::WatchlistEntryRemoved { .. }))
    );
}

#[tokio::test]
async fn fails_when_not_on_watchlist() {
    let b = TestContextBuilder::new();
    let deps = RemoveFromWatchlistDeps {
        watchlist: b.watchlist_repo.clone(),
        event_publisher: b.event_publisher.clone(),
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
