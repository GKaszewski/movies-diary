use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    value_objects::{ExternalMetadataId, MovieId, PosterPath, Rating, ReviewId, UserId},
};

#[derive(Clone, Debug)]
pub enum DomainEvent {
    ReviewLogged {
        review_id: ReviewId,
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        watched_at: NaiveDateTime,
    },
    ReviewUpdated {
        review_id: ReviewId,
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        watched_at: NaiveDateTime,
    },
    MovieDiscovered {
        movie_id: MovieId,
        external_metadata_id: ExternalMetadataId,
    },
    MovieDeleted {
        movie_id: MovieId,
        poster_path: Option<PosterPath>,
    },
    UserUpdated {
        user_id: UserId,
    },
    ReviewDeleted {
        review_id: ReviewId,
        user_id: UserId,
    },
    MovieEnrichmentRequested {
        movie_id: MovieId,
        external_metadata_id: String,
    },
    ImageStored {
        key: String,
    },
    WatchlistEntryAdded {
        user_id: UserId,
        movie_id: MovieId,
        movie_title: String,
        release_year: u16,
        external_metadata_id: Option<String>,
        added_at: chrono::NaiveDateTime,
    },
    WatchlistEntryRemoved {
        user_id: UserId,
        movie_id: MovieId,
    },
}

#[async_trait]
pub trait AckHandle: Send + Sync {
    async fn ack(&self) -> Result<(), DomainError>;
    async fn nack(&self) -> Result<(), DomainError>;
}

pub struct EventEnvelope {
    pub event: DomainEvent,
    ack: Box<dyn AckHandle>,
}

impl EventEnvelope {
    pub fn new(event: DomainEvent, ack: Box<dyn AckHandle>) -> Self {
        Self { event, ack }
    }

    pub async fn ack(self) -> Result<(), DomainError> {
        self.ack.ack().await
    }

    pub async fn nack(self) -> Result<(), DomainError> {
        self.ack.nack().await
    }
}
