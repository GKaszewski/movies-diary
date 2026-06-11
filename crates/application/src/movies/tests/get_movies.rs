use domain::testing::InMemoryMovieRepository;

use crate::movies::{deps::GetMoviesDeps, get_movies, queries::GetMoviesQuery};

#[tokio::test]
async fn returns_empty_when_no_movies() {
    let deps = GetMoviesDeps {
        movie: InMemoryMovieRepository::new(),
    };

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
