use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionCreatedResponse {
    pub session_id: String,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionStateResponse {
    pub session_id: String,
    pub columns: Vec<String>,
    pub has_mappings: bool,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiFieldMapping {
    pub source_column: String,
    pub domain_field: String,
    pub rating_scale: Option<f64>,
    pub date_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApplyMappingRequest {
    pub mappings: Vec<ApiFieldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConfirmRequest {
    pub confirmed_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SaveProfileRequest {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PreviewRowData {
    pub index: usize,
    pub title: Option<String>,
    pub release_year: Option<String>,
    pub director: Option<String>,
    pub rating: Option<String>,
    pub watched_at: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "status")]
pub enum PreviewRowDto {
    #[serde(rename = "valid")]
    Valid(PreviewRowData),
    #[serde(rename = "duplicate")]
    Duplicate(PreviewRowData),
    #[serde(rename = "invalid")]
    Invalid { index: usize, errors: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImportPreviewResponse {
    pub rows: Vec<PreviewRowDto>,
}
