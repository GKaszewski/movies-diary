use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    value_objects::{GoalId, UserId},
};

use super::GoalType;

#[derive(Clone, Debug)]
pub struct Goal {
    id: GoalId,
    user_id: UserId,
    year: u16,
    target_count: u32,
    goal_type: GoalType,
    created_at: NaiveDateTime,
}

impl Goal {
    pub fn new(
        user_id: UserId,
        year: u16,
        target_count: u32,
        goal_type: GoalType,
    ) -> Result<Self, DomainError> {
        if year < 2020 {
            return Err(DomainError::ValidationError(
                "Goal year must be 2020 or later".into(),
            ));
        }
        if target_count < 1 {
            return Err(DomainError::ValidationError(
                "Target count must be at least 1".into(),
            ));
        }
        Ok(Self {
            id: GoalId::generate(),
            user_id,
            year,
            target_count,
            goal_type,
            created_at: chrono::Utc::now().naive_utc(),
        })
    }

    pub fn from_persistence(
        id: GoalId,
        user_id: UserId,
        year: u16,
        target_count: u32,
        goal_type: GoalType,
        created_at: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            year,
            target_count,
            goal_type,
            created_at,
        }
    }

    pub fn update_target(&mut self, target_count: u32) -> Result<(), DomainError> {
        if target_count < 1 {
            return Err(DomainError::ValidationError(
                "Target count must be at least 1".into(),
            ));
        }
        self.target_count = target_count;
        Ok(())
    }

    pub fn id(&self) -> &GoalId {
        &self.id
    }
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    pub fn year(&self) -> u16 {
        self.year
    }
    pub fn target_count(&self) -> u32 {
        self.target_count
    }
    pub fn goal_type(&self) -> &GoalType {
        &self.goal_type
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.created_at
    }
}

pub struct GoalWithProgress {
    pub goal: Goal,
    pub current_count: u32,
}

impl GoalWithProgress {
    pub fn percentage(&self) -> f64 {
        if self.goal.target_count == 0 {
            return 100.0;
        }
        ((self.current_count as f64 / self.goal.target_count as f64) * 100.0).min(100.0)
    }

    pub fn is_complete(&self) -> bool {
        self.current_count >= self.goal.target_count
    }
}

#[cfg(test)]
#[path = "tests/goal.rs"]
mod tests;
