use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::movies::{MovieDto, ReviewDto};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogReviewRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_metadata_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_release_year: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_director: Option<String>,
    pub rating: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub watched_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_medium: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DiaryEntryDto {
    pub movie: MovieDto,
    pub review: ReviewDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DiaryResponse {
    pub items: Vec<DiaryEntryDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DiaryQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub movie_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ActivityFeedQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FeedEntryDto {
    pub movie: MovieDto,
    pub review: ReviewDto,
    pub user_id: Uuid,
    pub user_display_name: String,
    pub is_federated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivityFeedResponse {
    pub items: Vec<FeedEntryDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExportQueryParams {
    /// Output format: `csv` (default) or `json`
    #[serde(default = "default_export_format")]
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EditReviewRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watched_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch_medium: Option<Option<String>>,
}

fn default_export_format() -> String {
    "csv".to_string()
}
