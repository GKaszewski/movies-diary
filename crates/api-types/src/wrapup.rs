use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GenerateWrapUpRequest {
    pub start_date: String,
    pub end_date: String,
    pub global: Option<bool>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WrapUpGeneratedResponse {
    pub id: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WrapUpStatusResponse {
    pub id: String,
    pub user_id: Option<String>,
    pub status: String,
    pub start_date: String,
    pub end_date: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WrapUpListResponse {
    pub items: Vec<WrapUpStatusResponse>,
}
