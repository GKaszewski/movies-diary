use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{ExternalPersonId, PersonId},
    value_objects::{
        ExternalMetadataId, GoalId, MovieId, PosterPath, Rating, ReviewId, UserId, WrapUpId,
    },
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
        external_metadata_id: ExternalMetadataId,
    },
    PersonEnrichmentRequested {
        person_id: PersonId,
        external_person_id: ExternalPersonId,
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
    FollowAccepted {
        local_user_id: UserId,
        remote_actor_url: String,
        outbox_url: String,
    },
    BackfillFollower {
        owner_user_id: UserId,
        follower_inbox_url: String,
    },
    FederationDeliveryRequested {
        inbox_url: String,
        activity_json: String,
        signing_actor_id: uuid::Uuid,
    },
    WatchEventIngested {
        user_id: UserId,
        title: String,
        source: String,
    },
    WrapUpRequested {
        wrapup_id: WrapUpId,
        user_id: Option<UserId>,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    },
    WrapUpCompleted {
        wrapup_id: WrapUpId,
    },
    SearchReindexRequested,
    PosterSynced {
        movie_id: MovieId,
    },
    GoalCreated {
        goal_id: GoalId,
        user_id: UserId,
        year: u16,
        target_count: u32,
    },
    GoalUpdated {
        goal_id: GoalId,
        user_id: UserId,
        year: u16,
        target_count: u32,
    },
    GoalDeleted {
        goal_id: GoalId,
        user_id: UserId,
        year: u16,
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

#[cfg(test)]
#[path = "tests/events.rs"]
mod tests;
