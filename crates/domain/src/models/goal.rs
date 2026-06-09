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
mod tests {
    use super::*;
    use crate::value_objects::UserId;

    fn make_goal(year: u16, target: u32) -> Result<Goal, DomainError> {
        Goal::new(UserId::generate(), year, target, GoalType::Movies)
    }

    #[test]
    fn new_goal_valid() {
        let g = make_goal(2024, 52);
        assert!(g.is_ok());
        let g = g.unwrap();
        assert_eq!(g.year(), 2024);
        assert_eq!(g.target_count(), 52);
    }

    #[test]
    fn new_goal_rejects_year_before_2020() {
        assert!(make_goal(2019, 10).is_err());
    }

    #[test]
    fn new_goal_rejects_zero_target() {
        assert!(make_goal(2024, 0).is_err());
    }

    #[test]
    fn update_target_valid() {
        let mut g = make_goal(2024, 10).unwrap();
        assert!(g.update_target(50).is_ok());
        assert_eq!(g.target_count(), 50);
    }

    #[test]
    fn update_target_rejects_zero() {
        let mut g = make_goal(2024, 10).unwrap();
        assert!(g.update_target(0).is_err());
    }

    #[test]
    fn from_persistence_preserves_fields() {
        let id = GoalId::generate();
        let uid = UserId::generate();
        let ts = chrono::Utc::now().naive_utc();
        let g = Goal::from_persistence(id.clone(), uid.clone(), 2025, 42, GoalType::Movies, ts);
        assert_eq!(*g.id(), id);
        assert_eq!(*g.user_id(), uid);
        assert_eq!(g.year(), 2025);
        assert_eq!(g.target_count(), 42);
        assert_eq!(g.created_at(), &ts);
    }

    #[test]
    fn percentage_calculation() {
        let g = make_goal(2024, 100).unwrap();
        let wp = GoalWithProgress {
            goal: g,
            current_count: 50,
        };
        assert!((wp.percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_caps_at_100() {
        let g = make_goal(2024, 10).unwrap();
        let wp = GoalWithProgress {
            goal: g,
            current_count: 20,
        };
        assert!((wp.percentage() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn is_complete() {
        let g = make_goal(2024, 10).unwrap();
        let wp = GoalWithProgress {
            goal: g,
            current_count: 10,
        };
        assert!(wp.is_complete());
    }

    #[test]
    fn is_not_complete() {
        let g = make_goal(2024, 10).unwrap();
        let wp = GoalWithProgress {
            goal: g,
            current_count: 9,
        };
        assert!(!wp.is_complete());
    }
}
