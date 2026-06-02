use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GenerateWrapUpRequest {
    pub start_date: String,
    pub end_date: String,
    pub user_id: Option<Uuid>,
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
