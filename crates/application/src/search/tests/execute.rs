use domain::models::SearchQuery;

use crate::search::execute;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_results() {
    let ctx = TestContextBuilder::new().build();

    let result = execute::execute(&ctx, SearchQuery::default())
        .await
        .unwrap();

    assert!(result.movies.items.is_empty());
    assert!(result.people.items.is_empty());
}
