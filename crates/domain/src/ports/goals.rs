use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::Goal,
    value_objects::{GoalId, UserId},
};

#[async_trait]
pub trait GoalCommand: Send + Sync {
    async fn save(&self, goal: &Goal) -> Result<(), DomainError>;
    async fn update(&self, goal: &Goal) -> Result<(), DomainError>;
    async fn delete(&self, id: &GoalId, user_id: &UserId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait GoalQuery: Send + Sync {
    async fn find_by_user_and_year(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<Goal>, DomainError>;
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError>;
}
