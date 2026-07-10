use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    models::WatchlistEntry,
    models::{MetadataSearchCriteria, Movie},
    ports::{MetadataClient, MovieCommand, WatchlistRepository},
    testing::{
        FakeMetadataClient, InMemoryMovieRepository, InMemoryReviewRepository,
        InMemoryWatchlistRepository, NoopEventPublisher,
    },
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterUrl, ReleaseYear, UserId},
};
use uuid::Uuid;

use crate::diary::commands::{LogReviewCommand, MovieInput};
use crate::diary::review_logger::DefaultReviewLogger;
use crate::ports::ReviewLogger;

fn make_logger(
    movies: &Arc<InMemoryMovieRepository>,
    reviews: &Arc<InMemoryReviewRepository>,
    watchlist: &Arc<InMemoryWatchlistRepository>,
    events: &Arc<NoopEventPublisher>,
) -> DefaultReviewLogger {
    DefaultReviewLogger::new(
        Arc::clone(movies) as _,
        Arc::clone(movies) as _,
        Arc::clone(reviews) as _,
        Arc::clone(watchlist) as _,
        Arc::new(FakeMetadataClient) as _,
        Arc::clone(events) as _,
    )
}

#[tokio::test]
async fn logs_review_with_manual_movie() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let uid = Uuid::new_v4();
    let cmd = LogReviewCommand {
        user_id: uid,
        input: MovieInput {
            movie_id: None,
            external_metadata_id: None,
            manual_title: Some("Test Film".into()),
            manual_release_year: Some(2024),
            manual_director: None,
        },
        rating: 4,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    logger.log_review(cmd).await.unwrap();

    assert_eq!(movies.count(), 1);
    assert_eq!(reviews.count(), 1);
}

#[tokio::test]
async fn removes_from_watchlist_on_review() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let uid = Uuid::new_v4();
    let user_id = UserId::from_uuid(uid);

    // Create and store movie
    let movie = Movie::new(
        None,
        MovieTitle::new("Watchlisted Film".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    // Add to watchlist
    let entry = WatchlistEntry::new(user_id.clone(), MovieId::from_uuid(movie_id));
    watchlist.add(&entry).await.unwrap();
    assert_eq!(watchlist.count(), 1);

    // Log review for same movie
    let cmd = LogReviewCommand {
        user_id: uid,
        input: MovieInput {
            movie_id: Some(movie_id),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
        rating: 5,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    logger.log_review(cmd).await.unwrap();

    assert_eq!(watchlist.count(), 0);
}

#[tokio::test]
async fn logs_review_with_existing_movie_by_id() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let movie = Movie::new(
        None,
        MovieTitle::new("Existing Film".into()).unwrap(),
        ReleaseYear::new(2020).unwrap(),
        None,
        None,
    );
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    let cmd = LogReviewCommand {
        user_id: Uuid::new_v4(),
        input: MovieInput {
            movie_id: Some(movie_uuid),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
        rating: 3,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    logger.log_review(cmd).await.unwrap();

    assert_eq!(movies.count(), 1);
    assert_eq!(reviews.count(), 1);
}

#[tokio::test]
async fn existing_movie_not_found_returns_error() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let cmd = LogReviewCommand {
        user_id: Uuid::new_v4(),
        input: MovieInput {
            movie_id: Some(Uuid::new_v4()),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        },
        rating: 4,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    assert!(logger.log_review(cmd).await.is_err());
}

#[tokio::test]
async fn invalid_rating_returns_error() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let cmd = LogReviewCommand {
        user_id: Uuid::new_v4(),
        input: MovieInput {
            movie_id: None,
            external_metadata_id: None,
            manual_title: Some("Film".into()),
            manual_release_year: Some(2024),
            manual_director: None,
        },
        rating: 6,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    let result = logger.log_review(cmd).await;
    assert!(result.is_err());
    // No repo calls should have happened
    assert_eq!(movies.count(), 0);
    assert_eq!(reviews.count(), 0);
}

#[tokio::test]
async fn watchlist_not_present_does_not_publish_removed() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();
    let logger = make_logger(&movies, &reviews, &watchlist, &events);

    let cmd = LogReviewCommand {
        user_id: Uuid::new_v4(),
        input: MovieInput {
            movie_id: None,
            external_metadata_id: None,
            manual_title: Some("No Watchlist Film".into()),
            manual_release_year: Some(2024),
            manual_director: None,
        },
        rating: 4,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    logger.log_review(cmd).await.unwrap();

    let published = events.published();
    assert!(
        !published
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::WatchlistEntryRemoved { .. })),
        "should not publish WatchlistEntryRemoved when not on watchlist"
    );
}

/// A metadata client that returns a movie with an external_metadata_id,
/// triggering the MovieDiscovered event path.
struct MetadataClientWithExternalId;

#[async_trait]
impl MetadataClient for MetadataClientWithExternalId {
    async fn fetch_movie_metadata(
        &self,
        _criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        Ok(Movie::new(
            Some(ExternalMetadataId::new("tmdb:99999".into()).unwrap()),
            MovieTitle::new("Discovered Film".into()).unwrap(),
            ReleaseYear::new(2024).unwrap(),
            None,
            None,
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
async fn publishes_movie_discovered_for_new_movie_with_external_id() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let watchlist = InMemoryWatchlistRepository::new();
    let events = NoopEventPublisher::new();

    let logger = DefaultReviewLogger::new(
        Arc::clone(&movies) as _,
        Arc::clone(&movies) as _,
        Arc::clone(&reviews) as _,
        Arc::clone(&watchlist) as _,
        Arc::new(MetadataClientWithExternalId) as _,
        Arc::clone(&events) as _,
    );

    let cmd = LogReviewCommand {
        user_id: Uuid::new_v4(),
        input: MovieInput {
            movie_id: None,
            external_metadata_id: None,
            manual_title: Some("Discovered Film".into()),
            manual_release_year: Some(2024),
            manual_director: None,
        },
        rating: 5,
        comment: None,
        watched_at: Utc::now().naive_utc(),
        watch_medium: None,
    };

    logger.log_review(cmd).await.unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::MovieDiscovered { .. })),
        "should publish MovieDiscovered for new movie with external_metadata_id"
    );
    assert!(
        published.iter().any(|e| matches!(
            e,
            domain::events::DomainEvent::MovieEnrichmentRequested { .. }
        )),
        "should publish MovieEnrichmentRequested"
    );
}
