use domain::testing::InMemoryStore;

use application::integrations::cleanup;

#[tokio::test]
async fn returns_zero_when_nothing_to_clean() {
    let watch_events = InMemoryStore::new();

    let count = cleanup::execute(watch_events).await.unwrap();

    assert_eq!(count, 0);
}
