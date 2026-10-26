use crate::support::{log_review_command, log_review_command_manual};
use domain::testing::{test_movie_with_external_id, test_movie_with_year};
use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::WatchlistEntry,
    models::{MetadataSearchCriteria, Movie},
    ports::{MetadataClient, MovieCommand, WatchlistRepository},
    testing::{FakeMetadataClient, InMemoryStore},
    value_objects::{ExternalMetadataId, MovieId, PosterUrl, UserId},
};
use uuid::Uuid;

use application::diary::commands::LogReviewCommand;
use application::diary::review_logger::DefaultReviewLogger;
use application::ports::ReviewLogger;

fn make_logger(
    movies: &Arc<InMemoryStore>,
    reviews: &Arc<InMemoryStore>,
    watchlist: &Arc<InMemoryStore>,
) -> DefaultReviewLogger {
    DefaultReviewLogger::new(
        Arc::clone(movies) as _,
        Arc::clone(movies) as _,
        Arc::clone(reviews) as _,
        Arc::clone(watchlist) as _,
        Arc::new(FakeMetadataClient) as _,
    )
}

#[tokio::test]
async fn logs_review_with_manual_movie() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let uid = Uuid::new_v4();
    let cmd = log_review_command_manual(uid, "Test Film", 2024);

    logger.log_review(cmd).await.unwrap();

    assert_eq!(movies.movie_count(), 1);
    assert_eq!(reviews.review_count(), 1);
}

#[tokio::test]
async fn removes_from_watchlist_on_review() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let uid = Uuid::new_v4();
    let user_id = UserId::from_uuid(uid);

    // Create and store movie
    let movie = test_movie_with_year("Watchlisted Film", 2024);
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    // Add to watchlist
    let entry = WatchlistEntry::new(user_id.clone(), MovieId::from_uuid(movie_id));
    watchlist.add(&entry, &[]).await.unwrap();
    assert_eq!(watchlist.watchlist_count(), 1);

    // Log review for same movie
    let cmd = LogReviewCommand {
        rating: 5,
        ..log_review_command(uid, movie_id)
    };

    logger.log_review(cmd).await.unwrap();

    assert_eq!(watchlist.watchlist_count(), 0);
    assert!(
        movies
            .recorded_events()
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::WatchlistEntryRemoved { .. })),
        "the removal event rides the watchlist write"
    );
}

#[tokio::test]
async fn logs_review_with_existing_movie_by_id() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let movie = test_movie_with_year("Existing Film", 2020);
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let cmd = LogReviewCommand {
        rating: 3,
        ..log_review_command(Uuid::new_v4(), movie_uuid)
    };

    logger.log_review(cmd).await.unwrap();

    assert_eq!(movies.movie_count(), 1);
    assert_eq!(reviews.review_count(), 1);
}

#[tokio::test]
async fn existing_movie_not_found_returns_error() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let cmd = log_review_command(Uuid::new_v4(), Uuid::new_v4());

    assert!(logger.log_review(cmd).await.is_err());
}

#[tokio::test]
async fn invalid_rating_returns_error() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let cmd = LogReviewCommand {
        rating: 6,
        ..log_review_command_manual(Uuid::new_v4(), "Film", 2024)
    };

    let result = logger.log_review(cmd).await;
    assert!(result.is_err());
    // No repo calls should have happened
    assert_eq!(movies.movie_count(), 0);
    assert_eq!(reviews.review_count(), 0);
}

#[tokio::test]
async fn watchlist_not_present_does_not_publish_removed() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);
    let logger = make_logger(&movies, &reviews, &watchlist);

    let cmd = log_review_command_manual(Uuid::new_v4(), "No Watchlist Film", 2024);

    logger.log_review(cmd).await.unwrap();

    let published = movies.recorded_events();
    assert!(
        !published
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::WatchlistEntryRemoved { .. })),
        "should not publish WatchlistEntryRemoved when not on watchlist"
    );
}

struct MetadataClientWithExternalId;

#[async_trait]
impl MetadataClient for MetadataClientWithExternalId {
    async fn fetch_movie_metadata(
        &self,
        _criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        Ok(test_movie_with_external_id(
            "Discovered Film",
            2024,
            ExternalMetadataId::new("tmdb:99999".into()).unwrap(),
        ))
    }

    async fn get_poster_url(
        &self,
        _external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        Ok(None)
    }
}

#[tokio::test]
async fn movie_discovered_rides_the_movie_write_for_a_new_movie_with_external_id() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let watchlist = Arc::clone(&movies);

    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&reviews) as _,
        Arc::clone(&watchlist) as _,
        Arc::new(MetadataClientWithExternalId) as _,
    );

    let cmd = LogReviewCommand {
        rating: 5,
        ..log_review_command_manual(Uuid::new_v4(), "Discovered Film", 2024)
    };

    logger.log_review(cmd).await.unwrap();

    let published = movies.recorded_events();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::MovieDiscovered { .. })),
        "MovieDiscovered must be recorded with the movie write"
    );
}

#[tokio::test]
async fn logging_a_review_fails_when_the_review_store_refuses() {
    let movies = InMemoryStore::new();
    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::new(domain::testing::RefusingReviewRepository),
        Arc::clone(&movies) as _,
        Arc::new(FakeMetadataClient),
    );

    let result = logger
        .log_review(log_review_command_manual(Uuid::new_v4(), "Test Film", 2024))
        .await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
}

#[tokio::test]
async fn logging_a_review_saves_the_review_and_its_event_together() {
    let movies = InMemoryStore::new();
    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::new(FakeMetadataClient),
    );

    logger
        .log_review(log_review_command_manual(Uuid::new_v4(), "Test Film", 2024))
        .await
        .unwrap();

    assert_eq!(movies.review_count(), 1);
    let recorded = movies.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        domain::events::DomainEvent::ReviewLogged { .. }
    ));
}

#[tokio::test]
async fn a_refused_save_propagates_and_records_nothing() {
    let movies = InMemoryStore::new();
    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::new(domain::testing::RefusingReviewRepository),
        Arc::clone(&movies) as _,
        Arc::new(FakeMetadataClient),
    );

    let result = logger
        .log_review(log_review_command_manual(Uuid::new_v4(), "Test Film", 2024))
        .await;

    assert!(result.is_err());
    assert!(movies.recorded_events().is_empty());
}

#[tokio::test]
async fn enrichment_request_rides_the_review_save() {
    let movies = InMemoryStore::new();
    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::new(MetadataClientWithExternalId),
    );

    logger
        .log_review(log_review_command_manual(
            Uuid::new_v4(),
            "Discovered Film",
            2024,
        ))
        .await
        .unwrap();

    let recorded = movies.recorded_events();
    assert!(recorded.iter().any(|e| matches!(
        e,
        domain::events::DomainEvent::MovieEnrichmentRequested { .. }
    )));
    assert!(
        recorded
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::ReviewLogged { .. }))
    );
}
