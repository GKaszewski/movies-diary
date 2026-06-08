use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GoalDto {
    pub year: u16,
    pub target_count: u32,
    pub current_count: u32,
    pub percentage: f64,
    pub is_complete: bool,
    pub goal_type: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSettingsDto {
    pub federate_goals: bool,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserSettingsRequest {
    pub federate_goals: bool,
}
