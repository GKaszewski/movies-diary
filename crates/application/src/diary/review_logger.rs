use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::Review,
    ports::{
        EventPublisher, MetadataClient, MovieRepository, ReviewRepository, WatchlistRepository,
    },
    value_objects::{Comment, Rating, UserId},
};

use crate::diary::commands::LogReviewCommand;
use crate::movies::resolve::resolve_and_persist_movie;
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

        let (movie, is_new_movie) = resolve_and_persist_movie(
            &cmd.input,
            self.movie_repo.as_ref(),
            self.metadata_client.as_ref(),
            self.event_publisher.as_ref(),
        )
        .await?;

        // Always upsert: even existing movies may have updated metadata
        if !is_new_movie {
            self.movie_repo.upsert_movie(&movie).await?;
        }

        let review = Review::new(
            movie.id().clone(),
            user_id,
            rating,
            comment,
            cmd.watched_at,
            cmd.watch_medium,
        )?;
        self.review_repo.save_review(&review).await?;
        let review_event = DomainEvent::ReviewLogged {
            review_id: review.id().clone(),
            movie_id: review.movie_id().clone(),
            user_id: review.user_id().clone(),
            rating: review.rating().clone(),
            watched_at: *review.watched_at(),
        };

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

        if let Some(ext_id) = movie.external_metadata_id() {
            self.event_publisher
                .publish(&DomainEvent::MovieEnrichmentRequested {
                    movie_id: movie.id().clone(),
                    external_metadata_id: ext_id.clone(),
                })
                .await?;
        }

        self.event_publisher.publish(&review_event).await
    }
}

#[cfg(test)]
#[path = "tests/review_logger.rs"]
mod tests;
