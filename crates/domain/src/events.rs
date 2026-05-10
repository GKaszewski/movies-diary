use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    value_objects::{ExternalMetadataId, MovieId, Rating, ReviewId, UserId},
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
