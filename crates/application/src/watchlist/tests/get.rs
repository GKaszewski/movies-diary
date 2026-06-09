use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::watchlist::{get, queries::GetWatchlistQuery};

#[tokio::test]
async fn returns_empty_page_for_new_user() {
    let ctx = TestContextBuilder::new().build();
    let result = get::execute(
        &ctx,
        GetWatchlistQuery {
            user_id: Uuid::new_v4(),
            limit: None,
            offset: None,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
    assert_eq!(result.total_count, 0);
}
