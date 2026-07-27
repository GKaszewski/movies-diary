use async_trait::async_trait;

use crate::{
    errors::DomainError,
    value_objects::{FollowStatus, SocialActor},
};

#[async_trait]
pub trait FollowCommand: Send + Sync {
    async fn add_follow(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError>;

    async fn update_follow_status(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError>;

    async fn remove_follow(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<(), DomainError>;

    async fn add_follower(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError>;

    async fn update_follower_status(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError>;

    async fn remove_follower_record(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait FollowQuery: Send + Sync {
    async fn get_following(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_followers(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError>;

    async fn count_following(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn count_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn is_following(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<bool, DomainError>;
}
