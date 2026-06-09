use crate::{diary::get_diary, diary::queries::GetDiaryQuery, test_helpers::TestContextBuilder};

#[tokio::test]
async fn returns_empty_page() {
    let ctx = TestContextBuilder::new().build();

    let result = get_diary::execute(
        &ctx,
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
