use std::sync::Arc;

use domain::{
    models::Movie,
    ports::MovieRepository,
    testing::{InMemoryMovieRepository, InMemoryWatchlistRepository},
    value_objects::{MovieTitle, ReleaseYear},
};

use crate::{
    diary::commands::MovieInput, test_helpers::TestContextBuilder, watchlist::add,
    watchlist::commands::AddToWatchlistCommand,
};

#[tokio::test]
async fn test_add_to_watchlist_resolves_and_saves() {
    let movies = InMemoryMovieRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();

    let movie = Movie::new(
        None,
        MovieTitle::new("The Thing".into()).unwrap(),
        ReleaseYear::new(1982).unwrap(),
        None,
        None,
    );
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_watchlist(Arc::clone(&watchlist) as _)
        .build();

    let cmd = AddToWatchlistCommand {
        user_id: uuid::Uuid::new_v4(),
        input: MovieInput {
            movie_id: Some(movie_uuid),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
    };

    add::execute(&ctx, cmd).await.unwrap();

    assert_eq!(watchlist.count(), 1);
}

#[tokio::test]
async fn test_add_to_watchlist_already_present_is_idempotent() {
    let movies = InMemoryMovieRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();

    let movie = Movie::new(
        None,
        MovieTitle::new("RoboCop".into()).unwrap(),
        ReleaseYear::new(1987).unwrap(),
        None,
        None,
    );
    let movie_uuid = movie.id().value();
    let user_id = uuid::Uuid::new_v4();
    movies.upsert_movie(&movie).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_watchlist(Arc::clone(&watchlist) as _)
        .build();

    let make_cmd = || AddToWatchlistCommand {
        user_id,
        input: MovieInput {
            movie_id: Some(movie_uuid),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
    };

    add::execute(&ctx, make_cmd()).await.unwrap();
    add::execute(&ctx, make_cmd()).await.unwrap();

    assert_eq!(watchlist.count(), 1, "idempotent add should not duplicate");
}

#[tokio::test]
async fn test_add_to_watchlist_with_manual_movie() {
    let movies = InMemoryMovieRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_watchlist(Arc::clone(&watchlist) as _)
        .build();

    let cmd = AddToWatchlistCommand {
        user_id: uuid::Uuid::new_v4(),
        input: MovieInput {
            movie_id: None,
            external_metadata_id: None,
            manual_title: Some("New Manual Movie".into()),
            manual_release_year: Some(2024),
            manual_director: None,
        },
    };

    add::execute(&ctx, cmd).await.unwrap();

    assert_eq!(watchlist.count(), 1);
    assert_eq!(movies.count(), 1);
}

#[tokio::test]
async fn test_add_to_watchlist_movie_not_found_by_id() {
    let movies = InMemoryMovieRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_watchlist(Arc::clone(&watchlist) as _)
        .build();

    let cmd = AddToWatchlistCommand {
        user_id: uuid::Uuid::new_v4(),
        input: MovieInput {
            movie_id: Some(uuid::Uuid::new_v4()),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
    };

    assert!(add::execute(&ctx, cmd).await.is_err());
}
