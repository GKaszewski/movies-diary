use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GoalDto {
    pub year: u16,
    pub target_count: u32,
    pub current_count: u32,
    pub percentage: f64,
    pub is_complete: bool,
    #[schema(value_type = String)]
    pub goal_type: domain::models::GoalType,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GoalsResponse {
    pub goals: Vec<GoalDto>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateGoalRequest {
    pub year: u16,
    pub target_count: u32,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateGoalRequest {
    pub target_count: u32,
}
