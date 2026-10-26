use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        Goal, Movie, ProfileField, Review, WatchEvent, WatchEventStatus, WatchlistEntry,
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::{
        EventPublisher, FollowCommand, FollowWrite, GoalCommand, MovieCommand, ReviewRepository,
        SocialCommand, UserProfileFieldsRepository, WatchEventCommand, WatchlistRepository,
    },
    value_objects::{
        FollowTarget, GoalId, MovieId, ReviewId, SocialIdentity, UserId, WatchEventId,
    },
};

fn refused() -> DomainError {
    DomainError::InfrastructureError("refused".into())
}

pub struct RefusingReviewRepository;

#[async_trait]
impl ReviewRepository for RefusingReviewRepository {
    async fn save_review(&self, _: &Review, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn get_review_by_id(&self, _: &ReviewId) -> Result<Option<Review>, DomainError> {
        Err(refused())
    }
    async fn update_review(&self, _: &Review, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn delete_review_and_orphaned_movie(
        &self,
        _: &ReviewId,
        _: &MovieId,
        _: &[DomainEvent],
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn get_all_reviews_for_user(&self, _: &UserId) -> Result<Vec<Review>, DomainError> {
        Err(refused())
    }
}

pub struct RefusingMovieCommand;

#[async_trait]
impl MovieCommand for RefusingMovieCommand {
    async fn upsert_movie(&self, _: &Movie, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn delete_movie(&self, _: &MovieId, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
}

pub struct RefusingWatchlistRepository;

#[async_trait]
impl WatchlistRepository for RefusingWatchlistRepository {
    async fn add(&self, _: &WatchlistEntry, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn remove(&self, _: &UserId, _: &MovieId, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn remove_if_present(
        &self,
        _: &UserId,
        _: &MovieId,
        _: &[DomainEvent],
    ) -> Result<bool, DomainError> {
        Err(refused())
    }
    async fn get_for_user(
        &self,
        _: &UserId,
        _: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        Err(refused())
    }
    async fn contains(&self, _: &UserId, _: &MovieId) -> Result<bool, DomainError> {
        Err(refused())
    }
}

pub struct RefusingGoalCommand;

#[async_trait]
impl GoalCommand for RefusingGoalCommand {
    async fn save(&self, _: &Goal, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn update(&self, _: &Goal, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn delete(&self, _: &GoalId, _: &UserId, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
}

pub struct RefusingProfileFieldsRepository;

#[async_trait]
impl UserProfileFieldsRepository for RefusingProfileFieldsRepository {
    async fn get_fields(&self, _: &UserId) -> Result<Vec<ProfileField>, DomainError> {
        Err(refused())
    }
    async fn set_fields(
        &self,
        _: &UserId,
        _: Vec<ProfileField>,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
}

pub struct RefusingEventPublisher;

#[async_trait]
impl EventPublisher for RefusingEventPublisher {
    async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
        Err(refused())
    }
}

pub struct RefusingWatchEventCommand;

#[async_trait]
impl WatchEventCommand for RefusingWatchEventCommand {
    async fn save(&self, _: &WatchEvent, _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn update_status(
        &self,
        _: &WatchEventId,
        _: WatchEventStatus,
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn update_status_batch(
        &self,
        _: &[WatchEventId],
        _: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        Err(refused())
    }
    async fn delete_non_pending_older_than(&self, _: NaiveDateTime) -> Result<u64, DomainError> {
        Err(refused())
    }
}

pub struct RefusingSocialCommand;

#[async_trait]
impl SocialCommand for RefusingSocialCommand {
    async fn follow(
        &self,
        _: &UserId,
        _: &FollowTarget,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn unfollow(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn accept_follow(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn reject_follow(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn remove_follower(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn block(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
    async fn unblock(
        &self,
        _: &UserId,
        _: &SocialIdentity,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(refused())
    }
}

pub struct RefusingFollowCommand;

#[async_trait]
impl FollowCommand for RefusingFollowCommand {
    async fn apply(&self, _: &[FollowWrite], _: &[DomainEvent]) -> Result<(), DomainError> {
        Err(refused())
    }
}
