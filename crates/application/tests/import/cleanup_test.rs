use std::sync::Arc;

use domain::testing::InMemoryStore;

use application::import::cleanup;

#[tokio::test]
async fn returns_zero_when_nothing_expired() {
    let sessions = InMemoryStore::new();

    let result = cleanup::execute(Arc::clone(&sessions) as _).await.unwrap();

    assert_eq!(result, 0);
}
