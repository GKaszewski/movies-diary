use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::watchlist::{get, queries::GetWatchlistQuery};

#[tokio::test]
async fn returns_empty_page_for_new_user() {
    let b = TestContextBuilder::new();
    let result = get::execute(
        b.watchlist_repo.clone(),
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
