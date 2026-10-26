use crate::support::TestContextBuilder;
use crate::support::{log_review_command, log_review_command_manual};
use domain::testing::test_movie_with_year;
use std::sync::Arc;

use domain::ports::MovieCommand;

use domain::testing::InMemoryStore;

use application::{
    diary::commands::LogReviewCommand, diary::log_review, diary::review_logger::DefaultReviewLogger,
};

fn build_logger(
    movies: &Arc<InMemoryStore>,
    reviews: &Arc<InMemoryStore>,
) -> Arc<dyn application::ports::ReviewLogger> {
    Arc::new(DefaultReviewLogger::new(
        Arc::clone(movies) as _,
        Arc::clone(movies) as _,
        Arc::clone(reviews) as _,
        TestContextBuilder::new().watchlist_repo,
        Arc::new(domain::testing::FakeMetadataClient) as _,
    ))
}

#[tokio::test]
async fn logging_a_review_creates_the_movie_and_the_review() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let logger = build_logger(&movies, &reviews);

    let user_id = uuid::Uuid::new_v4();
    let cmd = log_review_command_manual(user_id, "Blade Runner", 1982);

    log_review::execute(&logger, cmd).await.unwrap();

    assert_eq!(reviews.review_count(), 1, "review should be saved");
    assert!(
        !movies.recorded_events().is_empty(),
        "events should be recorded with the review"
    );
}

#[tokio::test]
async fn logging_a_review_reuses_an_existing_movie() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);

    let existing_movie = test_movie_with_year("Alien", 1979);
    let movie_uuid = existing_movie.id().value();
    movies.upsert_movie(&existing_movie, &[]).await.unwrap();

    let logger = build_logger(&movies, &reviews);

    let cmd = LogReviewCommand {
        rating: 5,
        ..log_review_command(uuid::Uuid::new_v4(), movie_uuid)
    };

    log_review::execute(&logger, cmd).await.unwrap();

    assert_eq!(movies.movie_count(), 1, "no duplicate movie");
    assert_eq!(reviews.review_count(), 1);
}

#[tokio::test]
async fn logging_a_review_with_an_invalid_rating_fails() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let logger = build_logger(&movies, &reviews);

    let cmd = LogReviewCommand {
        rating: 6,
        ..log_review_command_manual(uuid::Uuid::new_v4(), "Some Film", 2000)
    };
    let result = log_review::execute(&logger, cmd).await;
    assert!(result.is_err(), "rating > 5 should fail");
}
