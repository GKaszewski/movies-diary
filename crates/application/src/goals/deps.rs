use std::sync::Arc;

use domain::ports::{EventPublisher, GoalRepository, StatsRepository};

pub struct GoalCommandDeps {
    pub goal: Arc<dyn GoalRepository>,
    pub stats: Arc<dyn StatsRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct GoalQueryDeps {
    pub goal: Arc<dyn GoalRepository>,
    pub stats: Arc<dyn StatsRepository>,
}
