use std::sync::Arc;

use domain::ports::WatchEventRepository;
use domain::testing::InMemoryWatchEventRepository;

use crate::integrations::cleanup;

#[tokio::test]
async fn returns_zero_when_nothing_to_clean() {
    let watch_events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();

    let count = cleanup::execute(watch_events).await.unwrap();

    assert_eq!(count, 0);
}
