use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::movies::MovieDto;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WatchlistEntryDto {
    pub id: Uuid,
    pub movie: MovieDto,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WatchlistResponse {
    pub items: Vec<WatchlistEntryDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct AddToWatchlistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub movie_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_metadata_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_release_year: Option<u16>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WatchlistStatusResponse {
    pub on_watchlist: bool,
}
