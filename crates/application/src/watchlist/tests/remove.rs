use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::WatchlistEntry;
use domain::ports::WatchlistRepository;
use domain::testing::{InMemoryWatchlistRepository, NoopEventPublisher};
use domain::value_objects::{MovieId, UserId};
use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::watchlist::{commands::RemoveFromWatchlistCommand, remove};

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

    remove::execute(
        Arc::clone(&watchlist) as _,
        Arc::clone(&events) as _,
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
    let result = remove::execute(
        b.watchlist_repo.clone(),
        b.event_publisher.clone(),
        RemoveFromWatchlistCommand {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}
