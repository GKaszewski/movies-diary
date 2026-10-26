use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{FollowRelation, FollowStatus, SocialActor},
};

#[derive(Clone, Debug, PartialEq)]
pub enum FollowWrite {
    AddFollow {
        follower_id: uuid::Uuid,
        target_actor_url: String,
        status: FollowStatus,
    },
    UpdateFollowStatus {
        follower_id: uuid::Uuid,
        target_actor_url: String,
        status: FollowStatus,
    },
    RemoveFollow {
        follower_id: uuid::Uuid,
        target_actor_url: String,
    },
    AddFollower {
        local_user_id: uuid::Uuid,
        follower_actor_url: String,
        status: FollowStatus,
    },
    UpdateFollowerStatus {
        local_user_id: uuid::Uuid,
        follower_actor_url: String,
        status: FollowStatus,
    },
    RemoveFollowerRecord {
        local_user_id: uuid::Uuid,
        follower_actor_url: String,
    },
}

#[async_trait]
pub trait FollowCommand: Send + Sync {
    async fn apply(
        &self,
        writes: &[FollowWrite],
        events: &[DomainEvent],
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait FollowQuery: Send + Sync {
    async fn get_following(&self, user_id: uuid::Uuid) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_followers(&self, user_id: uuid::Uuid) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_pending_following(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<SocialActor>, DomainError>;

    async fn count_following(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn count_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn count_pending_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn get_relation(
        &self,
        viewer_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<FollowRelation, DomainError>;
}
