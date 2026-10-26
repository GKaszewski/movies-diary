use domain::testing::test_movie_with_year;
use std::sync::Arc;

use uuid::Uuid;

use domain::{
    ports::MovieCommand,
    testing::{FakeDiaryQuery, InMemoryStore},
};

use application::{
    diary::deps::GetMovieSocialPageDeps, diary::get_movie_social_page,
    diary::queries::GetMovieSocialPageQuery,
};

#[tokio::test]
async fn fails_when_movie_not_found() {
    let deps = GetMovieSocialPageDeps {
        movie_query: InMemoryStore::new(),
        diary: FakeDiaryQuery::new() as _,
        movie_profile: InMemoryStore::new(),
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
    let movies = InMemoryStore::new();

    let movie = test_movie_with_year("Social Movie", 2024);
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let deps = GetMovieSocialPageDeps {
        movie_query: Arc::clone(&movies) as _,
        diary: FakeDiaryQuery::new() as _,
        movie_profile: InMemoryStore::new(),
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
