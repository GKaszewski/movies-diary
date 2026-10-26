use domain::testing::InMemoryStore;

use application::movies::{deps::GetMoviesDeps, get_movies, queries::GetMoviesQuery};

#[tokio::test]
async fn returns_empty_when_no_movies() {
    let movie = InMemoryStore::new();
    let deps = GetMoviesDeps { movie };

    let result = get_movies::execute(
        &deps,
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
