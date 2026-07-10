use domain::testing::FakeDiaryQuery;
use std::sync::Arc;

use crate::{diary::get_diary, diary::queries::GetDiaryQuery};

#[tokio::test]
async fn returns_empty_page() {
    let diary = FakeDiaryQuery::new() as Arc<dyn domain::ports::DiaryQuery>;

    let result = get_diary::execute(
        &diary,
        GetDiaryQuery {
            limit: None,
            offset: None,
            sort_by: None,
            movie_id: None,
            user_id: None,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
    assert_eq!(result.total_count, 0);
}
