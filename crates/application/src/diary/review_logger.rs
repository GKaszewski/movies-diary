use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, Review},
    ports::{
        EventPublisher, MetadataClient, MovieRepository, ReviewRepository, WatchlistRepository,
    },
    value_objects::{Comment, MovieId, Rating, UserId},
};

use crate::diary::commands::LogReviewCommand;
use crate::diary::movie_resolver::{MovieResolver, MovieResolverDeps};
use crate::ports::ReviewLogger;

pub struct DefaultReviewLogger {
    movie_repo: Arc<dyn MovieRepository>,
    review_repo: Arc<dyn ReviewRepository>,
    watchlist_repo: Arc<dyn WatchlistRepository>,
    metadata_client: Arc<dyn MetadataClient>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl DefaultReviewLogger {
    pub fn new(
        movie_repo: Arc<dyn MovieRepository>,
        review_repo: Arc<dyn ReviewRepository>,
        watchlist_repo: Arc<dyn WatchlistRepository>,
        metadata_client: Arc<dyn MetadataClient>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            movie_repo,
            review_repo,
            watchlist_repo,
            metadata_client,
            event_publisher,
        }
    }
}

#[async_trait]
impl ReviewLogger for DefaultReviewLogger {
    async fn log_review(&self, cmd: LogReviewCommand) -> Result<(), DomainError> {
        let rating = Rating::new(cmd.rating)?;
        let user_id = UserId::from_uuid(cmd.user_id);
        let comment = cmd.comment.clone().map(Comment::new).transpose()?;

        let (movie, is_new_movie) = if let Some(id) = cmd.input.movie_id {
            let movie_id = MovieId::from_uuid(id);
            let movie = self
                .movie_repo
                .get_movie_by_id(&movie_id)
                .await?
                .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?;
            (movie, false)
        } else {
            let deps = MovieResolverDeps {
                repository: self.movie_repo.as_ref(),
                metadata_client: self.metadata_client.as_ref(),
            };
            MovieResolver::default_pipeline()
                .resolve(&cmd.input, &deps)
                .await?
        };

        self.movie_repo.upsert_movie(&movie).await?;

        let review = Review::new(movie.id().clone(), user_id, rating, comment, cmd.watched_at)?;
        let review_event = self.review_repo.save_review(&review).await?;

        let was_on_watchlist = self
            .watchlist_repo
            .remove_if_present(review.user_id(), review.movie_id())
            .await?;
        if was_on_watchlist {
            let _ = self
                .event_publisher
                .publish(&DomainEvent::WatchlistEntryRemoved {
                    user_id: review.user_id().clone(),
                    movie_id: review.movie_id().clone(),
                })
                .await;
        }

        publish_events(&self.event_publisher, &movie, is_new_movie, review_event).await
    }
}

async fn publish_events(
    publisher: &Arc<dyn EventPublisher>,
    movie: &Movie,
    is_new_movie: bool,
    review_event: DomainEvent,
) -> Result<(), DomainError> {
    if is_new_movie && let Some(ext_id) = movie.external_metadata_id() {
        publisher
            .publish(&DomainEvent::MovieDiscovered {
                movie_id: movie.id().clone(),
                external_metadata_id: ext_id.clone(),
            })
            .await?;
    }

    if let Some(ext_id) = movie.external_metadata_id() {
        publisher
            .publish(&DomainEvent::MovieEnrichmentRequested {
                movie_id: movie.id().clone(),
                external_metadata_id: ext_id.clone(),
            })
            .await?;
    }

    publisher.publish(&review_event).await
}

#[cfg(test)]
#[path = "tests/review_logger.rs"]
mod tests;
