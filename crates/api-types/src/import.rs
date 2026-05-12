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
    /// Column name in the source file
    pub source_column: String,
    /// Domain field: title | release_year | director | rating | watched_at | comment | external_metadata_id
    pub domain_field: String,
    /// For rating fields: multiply raw value by this factor (e.g. 0.5 for 10-point → 5-point scale)
    pub rating_scale: Option<f64>,
    /// For watched_at fields: strftime format hint (e.g. "%d/%m/%Y")
    pub date_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApplyMappingRequest {
    pub mappings: Vec<ApiFieldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConfirmRequest {
    /// Indices (0-based) of rows from the mapping preview to import
    pub confirmed_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SaveProfileRequest {
    /// Session UUID whose current field_mappings to save
    pub session_id: String,
    /// Human-readable profile name (e.g. "Letterboxd")
    pub name: String,
}
