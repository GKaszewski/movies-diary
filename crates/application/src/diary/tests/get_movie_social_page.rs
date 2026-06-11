use std::sync::Arc;

use uuid::Uuid;

use domain::{
    models::Movie,
    ports::MovieRepository,
    testing::{FakeDiaryRepository, InMemoryMovieProfileRepository, InMemoryMovieRepository},
    value_objects::{MovieTitle, ReleaseYear},
};

use crate::{
    diary::deps::GetMovieSocialPageDeps,
    diary::get_movie_social_page,
    diary::queries::GetMovieSocialPageQuery,
};

#[tokio::test]
async fn fails_when_movie_not_found() {
    let deps = GetMovieSocialPageDeps {
        movie: InMemoryMovieRepository::new(),
        diary: FakeDiaryRepository::new() as _,
        movie_profile: InMemoryMovieProfileRepository::new(),
    };

    let result = get_movie_social_page::execute(
        &deps,
        GetMovieSocialPageQuery {
            movie_id: Uuid::new_v4(),
            limit: 10,
            offset: 0,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn returns_movie_social_page() {
    let movies = InMemoryMovieRepository::new();

    let movie = Movie::new(
        None,
        MovieTitle::new("Social Movie".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    let deps = GetMovieSocialPageDeps {
        movie: Arc::clone(&movies) as _,
        diary: FakeDiaryRepository::new() as _,
        movie_profile: InMemoryMovieProfileRepository::new(),
    };

    let result = get_movie_social_page::execute(
        &deps,
        GetMovieSocialPageQuery {
            movie_id: movie_uuid,
            limit: 10,
            offset: 0,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.movie.title().value(), "Social Movie");
    assert_eq!(result.reviews.items.len(), 0);
}
