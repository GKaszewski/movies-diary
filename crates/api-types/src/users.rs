use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::diary::{DiaryEntryDto, DiaryResponse};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSummaryDto {
    pub id: Uuid,
    pub email: String,
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsersResponse {
    pub users: Vec<UserSummaryDto>,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UserProfileQueryParams {
    /// One of: `recent` (default), `ratings`, `history`, `trends`
    pub view: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserStatsDto {
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub favorite_director: Option<String>,
    pub most_active_month: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MonthActivityDto {
    pub year_month: String,
    pub month_label: String,
    pub count: i64,
    pub entries: Vec<DiaryEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MonthlyRatingDto {
    pub year_month: String,
    pub month_label: String,
    pub avg_rating: f64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DirectorStatDto {
    pub director: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserTrendsDto {
    pub monthly_ratings: Vec<MonthlyRatingDto>,
    pub top_directors: Vec<DirectorStatDto>,
    pub max_director_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserProfileResponse {
    pub user_id: Uuid,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_url: Option<String>,
    pub stats: UserStatsDto,
    pub following_count: usize,
    pub followers_count: usize,
    /// Populated for view=recent and view=ratings
    pub entries: Option<DiaryResponse>,
    /// Populated for view=history
    pub history: Option<Vec<MonthActivityDto>>,
    /// Populated for view=trends
    pub trends: Option<UserTrendsDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProfileResponse {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub also_known_as: Option<String>,
    pub fields: Vec<ProfileFieldDto>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProfileFieldDto {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateProfileFieldsRequest {
    pub fields: Vec<ProfileFieldDto>,
}
