use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::watchlist::{deps::GetWatchlistDeps, get, queries::GetWatchlistQuery};

#[tokio::test]
async fn returns_empty_page_for_new_user() {
    let b = TestContextBuilder::new();
    let deps = GetWatchlistDeps {
        watchlist: b.watchlist_repo.clone(),
    };
    let result = get::execute(
        &deps,
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
