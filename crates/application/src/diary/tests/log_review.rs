use std::sync::Arc;

use chrono::Utc;

use domain::{
    models::Movie,
    value_objects::{MovieTitle, ReleaseYear},
};

use domain::ports::MovieRepository;
use domain::testing::{InMemoryMovieRepository, InMemoryReviewRepository, NoopEventPublisher};

use crate::{
    diary::commands::{LogReviewCommand, MovieInput},
    diary::log_review,
    diary::review_logger::DefaultReviewLogger,
    test_helpers::TestContextBuilder,
};

fn build_logger(
    movies: &Arc<InMemoryMovieRepository>,
    reviews: &Arc<InMemoryReviewRepository>,
    events: &Arc<NoopEventPublisher>,
) -> Arc<dyn crate::ports::ReviewLogger> {
    Arc::new(DefaultReviewLogger::new(
        Arc::clone(movies) as _,
        Arc::clone(reviews) as _,
        TestContextBuilder::new().watchlist_repo,
        Arc::new(domain::testing::FakeMetadataClient) as _,
        Arc::clone(events) as _,
    ))
}

fn movie_input_manual(title: &str, year: u16) -> MovieInput {
    MovieInput {
        movie_id: None,
        external_metadata_id: None,
        manual_title: Some(title.to_string()),
        manual_release_year: Some(year),
        manual_director: None,
    }
}

fn movie_input_by_id(id: uuid::Uuid) -> MovieInput {
    MovieInput {
        movie_id: Some(id),
        external_metadata_id: None,
        manual_title: None,
        manual_release_year: None,
        manual_director: None,
    }
}

#[tokio::test]
async fn test_log_review_creates_movie_and_review() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let events = NoopEventPublisher::new();
    let logger = build_logger(&movies, &reviews, &events);

    let user_id = uuid::Uuid::new_v4();
    let cmd = LogReviewCommand {
        user_id,
        input: movie_input_manual("Blade Runner", 1982),
        rating: 4,
        comment: None,
        watched_at: Utc::now().naive_utc(),
    };

    log_review::execute(&logger, cmd).await.unwrap();

    assert_eq!(reviews.count(), 1, "review should be saved");
    assert!(!events.published().is_empty(), "events should be published");
}

#[tokio::test]
async fn test_log_review_reuses_existing_movie() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();

    let existing_movie = Movie::new(
        None,
        MovieTitle::new("Alien".into()).unwrap(),
        ReleaseYear::new(1979).unwrap(),
        None,
        None,
    );
    let movie_uuid = existing_movie.id().value();
    movies.upsert_movie(&existing_movie).await.unwrap();

    let events = NoopEventPublisher::new();
    let logger = build_logger(&movies, &reviews, &events);

    let cmd = LogReviewCommand {
        user_id: uuid::Uuid::new_v4(),
        input: movie_input_by_id(movie_uuid),
        rating: 5,
        comment: None,
        watched_at: Utc::now().naive_utc(),
    };

    log_review::execute(&logger, cmd).await.unwrap();

    assert_eq!(movies.count(), 1, "no duplicate movie");
    assert_eq!(reviews.count(), 1);
}

#[tokio::test]
async fn test_log_review_with_invalid_rating_fails() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let events = NoopEventPublisher::new();
    let logger = build_logger(&movies, &reviews, &events);

    let cmd = LogReviewCommand {
        user_id: uuid::Uuid::new_v4(),
        input: movie_input_manual("Some Film", 2000),
        rating: 6,
        comment: None,
        watched_at: Utc::now().naive_utc(),
    };
    let result = log_review::execute(&logger, cmd).await;
    assert!(result.is_err(), "rating > 5 should fail");
}
