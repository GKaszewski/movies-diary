use std::sync::Arc;

use domain::testing::InMemoryImportSessionRepository;

use crate::import::cleanup;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_zero_when_nothing_expired() {
    let sessions = InMemoryImportSessionRepository::new();
    let ctx = TestContextBuilder::new()
        .with_import_sessions(Arc::clone(&sessions) as _)
        .build();

    let result = cleanup::execute(&ctx).await.unwrap();

    assert_eq!(result, 0);
}
