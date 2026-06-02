use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, Review},
    value_objects::{Comment, MovieId, Rating, UserId},
};

use crate::{
    commands::LogReviewCommand,
    context::AppContext,
    movie_resolver::{MovieResolver, MovieResolverDeps},
};

pub async fn execute(ctx: &AppContext, cmd: LogReviewCommand) -> Result<(), DomainError> {
    let rating = Rating::new(cmd.rating)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    let comment = cmd.comment.clone().map(Comment::new).transpose()?;

    let (movie, is_new_movie) = if let Some(id) = cmd.input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        let movie = ctx
            .movie_repository
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?;
        (movie, false)
    } else {
        let deps = MovieResolverDeps {
            repository: ctx.movie_repository.as_ref(),
            metadata_client: ctx.metadata_client.as_ref(),
        };
        MovieResolver::default_pipeline()
            .resolve(&cmd.input, &deps)
            .await?
    };

    ctx.movie_repository.upsert_movie(&movie).await?;

    let review = Review::new(movie.id().clone(), user_id, rating, comment, cmd.watched_at)?;
    let review_event = ctx.review_repository.save_review(&review).await?;

    let was_on_watchlist = ctx
        .watchlist_repository
        .remove_if_present(review.user_id(), review.movie_id())
        .await?;
    if was_on_watchlist {
        let _ = ctx
            .event_publisher
            .publish(&DomainEvent::WatchlistEntryRemoved {
                user_id: review.user_id().clone(),
                movie_id: review.movie_id().clone(),
            })
            .await;
    }

    publish_events(ctx, &movie, is_new_movie, review_event).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use domain::{
        models::Movie,
        value_objects::{MovieTitle, ReleaseYear},
    };

    use domain::ports::MovieRepository;
    use domain::testing::{InMemoryMovieRepository, InMemoryReviewRepository, NoopEventPublisher};

    use crate::{
        commands::{LogReviewCommand, MovieInput},
        test_helpers::TestContextBuilder,
        use_cases::log_review,
    };

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
        let ctx = TestContextBuilder::new()
            .with_movies(Arc::clone(&movies) as _)
            .with_reviews(Arc::clone(&reviews) as _)
            .with_event_publisher(Arc::clone(&events) as _)
            .build();

        let user_id = uuid::Uuid::new_v4();
        let cmd = LogReviewCommand {
            user_id,
            input: movie_input_manual("Blade Runner", 1982),
            rating: 4,
            comment: None,
            watched_at: Utc::now().naive_utc(),
        };

        log_review::execute(&ctx, cmd).await.unwrap();

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

        let ctx = TestContextBuilder::new()
            .with_movies(Arc::clone(&movies) as _)
            .with_reviews(Arc::clone(&reviews) as _)
            .build();

        let cmd = LogReviewCommand {
            user_id: uuid::Uuid::new_v4(),
            input: movie_input_by_id(movie_uuid),
            rating: 5,
            comment: None,
            watched_at: Utc::now().naive_utc(),
        };

        log_review::execute(&ctx, cmd).await.unwrap();

        assert_eq!(movies.count(), 1, "no duplicate movie");
        assert_eq!(reviews.count(), 1);
    }

    #[tokio::test]
    async fn test_log_review_with_invalid_rating_fails() {
        let ctx = TestContextBuilder::new().build();
        let cmd = LogReviewCommand {
            user_id: uuid::Uuid::new_v4(),
            input: movie_input_manual("Some Film", 2000),
            rating: 6,
            comment: None,
            watched_at: Utc::now().naive_utc(),
        };
        let result = log_review::execute(&ctx, cmd).await;
        assert!(result.is_err(), "rating > 5 should fail");
    }
}

async fn publish_events(
    ctx: &AppContext,
    movie: &Movie,
    is_new_movie: bool,
    review_event: DomainEvent,
) -> Result<(), DomainError> {
    if is_new_movie && let Some(ext_id) = movie.external_metadata_id() {
        let discovery_event = DomainEvent::MovieDiscovered {
            movie_id: movie.id().clone(),
            external_metadata_id: ext_id.clone(),
        };
        ctx.event_publisher.publish(&discovery_event).await?;
    }

    if let Some(ext_id) = movie.external_metadata_id() {
        let enrichment_event = DomainEvent::MovieEnrichmentRequested {
            movie_id: movie.id().clone(),
            external_metadata_id: ext_id.value().to_string(),
        };
        ctx.event_publisher.publish(&enrichment_event).await?;
    }

    ctx.event_publisher.publish(&review_event).await?;
    Ok(())
}
