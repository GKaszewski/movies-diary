use crate::{
    movies::{get_movies, queries::GetMoviesQuery},
    test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn returns_empty_when_no_movies() {
    let ctx = TestContextBuilder::new().build();

    let result = get_movies::execute(
        &ctx,
        GetMoviesQuery {
            limit: None,
            offset: None,
            search: None,
            genre: None,
            language: None,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
}
