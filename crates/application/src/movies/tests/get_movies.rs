use domain::testing::InMemoryMovieRepository;

use crate::movies::{get_movies, queries::GetMoviesQuery};

#[tokio::test]
async fn returns_empty_when_no_movies() {
    let movie = InMemoryMovieRepository::new();

    let result = get_movies::execute(
        movie,
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
