use domain::models::SearchQuery;

use crate::search::deps::SearchDeps;
use crate::search::execute;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_results() {
    let b = TestContextBuilder::new();
    let deps = SearchDeps {
        search_port: b.search_port.clone(),
    };

    let result = execute::execute(&deps, SearchQuery::default())
        .await
        .unwrap();

    assert!(result.movies.items.is_empty());
    assert!(result.people.items.is_empty());
}
