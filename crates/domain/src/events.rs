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
    FollowRequested {
        follower: UserId,
        target: crate::value_objects::FollowTarget,
    },
    FollowAccepted {
        owner: UserId,
        requester: crate::value_objects::SocialIdentity,
    },
    FollowRejected {
        owner: UserId,
        requester: crate::value_objects::SocialIdentity,
    },
    Unfollowed {
        follower: UserId,
        target: crate::value_objects::SocialIdentity,
    },
    FollowerRemoved {
        owner: UserId,
        follower: crate::value_objects::SocialIdentity,
    },
    ActorBlocked {
        blocker: UserId,
        target: crate::value_objects::SocialIdentity,
    },
    ActorUnblocked {
        blocker: UserId,
        target: crate::value_objects::SocialIdentity,
    },
    BackfillFollower {
        owner_user_id: UserId,
        follower_inbox_url: String,
    },
    FederationDeliveryRequested {
        inbox_url: String,
        activity_json: serde_json::Value,
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
    UserDeleted {
        user_id: UserId,
    },
    UserAccountMoved {
        user_id: UserId,
        new_actor_url: String,
    },
}

#[async_trait]
pub trait AckHandle: Send + Sync {
    async fn ack(&self) -> Result<(), DomainError>;
    async fn nack(&self) -> Result<(), DomainError>;
    async fn defer(&self, delay: std::time::Duration) -> Result<(), DomainError>;
}

pub fn retry_backoff(attempt: u32) -> std::time::Duration {
    let secs = 5u64 * (1u64 << attempt.min(6));
    std::time::Duration::from_secs(secs.min(300))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EventId(uuid::Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    pub fn value(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EventEnvelope {
    pub event: DomainEvent,
    id: Option<EventId>,
    ack: Box<dyn AckHandle>,
}

impl EventEnvelope {
    pub fn new(id: Option<EventId>, event: DomainEvent, ack: Box<dyn AckHandle>) -> Self {
        Self { event, id, ack }
    }

    pub fn id(&self) -> Option<&EventId> {
        self.id.as_ref()
    }

    pub async fn ack(self) -> Result<(), DomainError> {
        self.ack.ack().await
    }

    pub async fn defer(self, delay: std::time::Duration) -> Result<(), DomainError> {
        self.ack.defer(delay).await
    }

    pub async fn nack(self) -> Result<(), DomainError> {
        self.ack.nack().await
    }
}
