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

    let ctx = TestContextBuilder::new()
        .with_watchlist(Arc::clone(&watchlist) as _)
        .with_event_publisher(Arc::clone(&events) as _)
        .build();

    remove::execute(
        &ctx,
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
    let ctx = TestContextBuilder::new().build();
    let result = remove::execute(
        &ctx,
        RemoveFromWatchlistCommand {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}
