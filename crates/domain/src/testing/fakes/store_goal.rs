use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::Goal,
    ports::{GoalCommand, GoalQuery},
    value_objects::{GoalId, UserId},
};

use super::InMemoryStore;

#[async_trait]
impl GoalCommand for InMemoryStore {
    async fn save(&self, goal: &Goal, events: &[DomainEvent]) -> Result<(), DomainError> {
        let mut goals = self.goals.write().unwrap();
        let duplicate = goals.values().any(|g| {
            g.id() == goal.id() || (g.user_id() == goal.user_id() && g.year() == goal.year())
        });
        if duplicate {
            return Err(DomainError::InfrastructureError(
                "Database operation failed".into(),
            ));
        }
        goals.insert(goal.id().value(), goal.clone());
        drop(goals);
        self.record_events(events);
        Ok(())
    }

    async fn update(&self, goal: &Goal, events: &[DomainEvent]) -> Result<(), DomainError> {
        let mut store = self.goals.write().unwrap();
        match store.entry(goal.id().value()) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                e.insert(goal.clone());
                drop(store);
                self.record_events(events);
                Ok(())
            }
            std::collections::hash_map::Entry::Vacant(_) => {
                Err(DomainError::NotFound("goal".into()))
            }
        }
    }

    async fn delete(
        &self,
        id: &GoalId,
        _user_id: &UserId,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        self.goals
            .write()
            .unwrap()
            .remove(&id.value())
            .ok_or_else(|| DomainError::NotFound("Goal not found".into()))?;
        self.record_events(events);
        Ok(())
    }
}

#[async_trait]
impl GoalQuery for InMemoryStore {
    async fn find_by_user_and_year(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<Goal>, DomainError> {
        let store = self.goals.read().unwrap();
        Ok(store
            .values()
            .find(|g| g.user_id().value() == user_id.value() && g.year() == year)
            .cloned())
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError> {
        let store = self.goals.read().unwrap();
        Ok(store
            .values()
            .filter(|g| g.user_id().value() == user_id.value())
            .cloned()
            .collect())
    }
}
