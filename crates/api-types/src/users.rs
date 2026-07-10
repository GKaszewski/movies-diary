use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::DiaryResponse;
use crate::diary::DiaryEntryDto;
use crate::goals::GoalDto;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSummaryDto {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: Option<String>,
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
    pub search: Option<String>,
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
pub struct UserProfileBase {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum ProfileViewData {
    Entries { entries: DiaryResponse },
    History { history: Vec<MonthActivityDto> },
    Trends { trends: UserTrendsDto },
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserProfileResponse {
    pub user_id: Uuid,
    #[serde(flatten)]
    pub profile: UserProfileBase,
    pub stats: UserStatsDto,
    pub following_count: usize,
    pub followers_count: usize,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub view_data: Option<ProfileViewData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goals: Option<Vec<GoalDto>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_federated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProfileResponse {
    #[serde(flatten)]
    pub profile: UserProfileBase,
    pub also_known_as: Option<String>,
    pub fields: Vec<ProfileFieldDto>,
    #[schema(value_type = String)]
    pub role: domain::models::UserRole,
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSettingsDto {
    pub federate_goals: bool,
    pub federate_reviews: bool,
    pub federate_watchlist: bool,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserSettingsRequest {
    pub federate_goals: bool,
    pub federate_reviews: bool,
    pub federate_watchlist: bool,
}
