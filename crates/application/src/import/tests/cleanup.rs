use std::sync::Arc;

use domain::testing::InMemoryImportSessionRepository;

use crate::import::cleanup;

#[tokio::test]
async fn returns_zero_when_nothing_expired() {
    let sessions = InMemoryImportSessionRepository::new();

    let result = cleanup::execute(Arc::clone(&sessions) as _).await.unwrap();

    assert_eq!(result, 0);
}
