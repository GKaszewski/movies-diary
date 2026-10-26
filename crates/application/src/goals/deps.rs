use std::sync::Arc;

use domain::ports::{GoalCommand, GoalQuery, StatsRepository};

pub struct GoalCommandDeps {
    pub goal_command: Arc<dyn GoalCommand>,
    pub goal_query: Arc<dyn GoalQuery>,
    pub stats: Arc<dyn StatsRepository>,
}

pub struct GoalQueryDeps {
    pub goal_query: Arc<dyn GoalQuery>,
    pub stats: Arc<dyn StatsRepository>,
}
