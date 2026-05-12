use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Movie list ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MoviesQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    /// Optional title filter (case-insensitive substring match)
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoviesResponse {
    pub items: Vec<MovieDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

// ── Movie profile (enrichment) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GenreDto {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct KeywordDto {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CastMemberDto {
    pub tmdb_person_id: u64,
    pub name: String,
    pub character: String,
    pub billing_order: u32,
    pub profile_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CrewMemberDto {
    pub tmdb_person_id: u64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieProfileResponse {
    pub tmdb_id: u64,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub budget_usd: Option<i64>,
    pub revenue_usd: Option<i64>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u32>,
    pub original_language: Option<String>,
    pub collection_name: Option<String>,
    pub genres: Vec<GenreDto>,
    pub keywords: Vec<KeywordDto>,
    pub cast: Vec<CastMemberDto>,
    pub crew: Vec<CrewMemberDto>,
    pub enriched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieDto {
    pub id: Uuid,
    pub title: String,
    pub release_year: u16,
    pub director: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewDto {
    pub id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewHistoryResponse {
    pub movie: MovieDto,
    pub viewings: Vec<ReviewDto>,
    pub trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieStatsDto {
    pub total_count: u64,
    pub avg_rating: Option<f64>,
    pub federated_count: u64,
    pub rating_histogram: [u64; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SocialReviewDto {
    pub user_display: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
    pub is_federated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SocialFeedResponse {
    pub items: Vec<SocialReviewDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieDetailResponse {
    pub movie: MovieDto,
    pub stats: MovieStatsDto,
    pub reviews: SocialFeedResponse,
}
