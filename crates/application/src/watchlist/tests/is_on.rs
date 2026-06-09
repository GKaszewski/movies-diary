use std::sync::Arc;

use domain::models::WatchlistEntry;
use domain::ports::WatchlistRepository;
use domain::testing::InMemoryWatchlistRepository;
use domain::value_objects::{MovieId, UserId};
use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::watchlist::{is_on, queries::IsOnWatchlistQuery};

#[tokio::test]
async fn returns_true_when_present() {
    let watchlist = InMemoryWatchlistRepository::new();
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
        .build();

    let result = is_on::execute(
        &ctx,
        IsOnWatchlistQuery {
            user_id: uid,
            movie_id: mid,
        },
    )
    .await
    .unwrap();

    assert!(result);
}

#[tokio::test]
async fn returns_false_when_absent() {
    let ctx = TestContextBuilder::new().build();
    let result = is_on::execute(
        &ctx,
        IsOnWatchlistQuery {
            user_id: Uuid::new_v4(),
            movie_id: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    assert!(!result);
}
