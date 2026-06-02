use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::value_objects::WrapUpId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovieRef {
    pub title: String,
    pub year: u16,
    pub runtime_minutes: Option<u32>,
    pub poster_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserRef {
    pub user_id: Uuid,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonStat {
    pub name: String,
    pub count: u32,
    pub avg_rating: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenreStat {
    pub genre: String,
    pub count: u32,
    pub avg_rating: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeywordStat {
    pub keyword: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LangStat {
    pub language: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonthCount {
    pub year_month: String,
    pub label: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WrapUpScope {
    User(Uuid),
    Global,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WrapUpReport {
    pub scope: WrapUpScope,
    pub date_range: DateRange,
    pub generated_at: chrono::DateTime<chrono::Utc>,

    // Core viewing
    pub total_movies: u32,
    pub total_watch_time_minutes: u32,
    pub movies_per_month: Vec<MonthCount>,
    pub busiest_month: Option<String>,
    pub busiest_day_of_week: Option<String>,
    pub avg_rating: Option<f64>,
    pub rating_distribution: [u32; 5],
    pub longest_movie: Option<MovieRef>,
    pub shortest_movie: Option<MovieRef>,

    // People insights
    pub top_directors: Vec<PersonStat>,
    pub top_actors: Vec<PersonStat>,
    pub director_diversity: u32,
    pub actor_diversity: u32,

    // Genre & taste
    pub top_genres: Vec<GenreStat>,
    pub genre_diversity: u32,
    pub highest_rated_genre: Option<String>,
    pub lowest_rated_genre: Option<String>,
    pub top_keywords: Vec<KeywordStat>,

    // Financial/production (None when data unavailable)
    pub total_budget_watched: Option<i64>,
    pub avg_budget: Option<i64>,
    pub language_distribution: Vec<LangStat>,
    pub oldest_movie: Option<MovieRef>,
    pub newest_movie: Option<MovieRef>,

    // Rewatch stats
    pub total_rewatches: u32,
    pub most_rewatched_movie: Option<MovieRef>,
    pub avg_rating_change_on_rewatch: Option<f64>,

    // Highlights
    pub highest_rated_movie: Option<MovieRef>,
    pub lowest_rated_movie: Option<MovieRef>,
    pub first_movie_of_period: Option<MovieRef>,
    pub last_movie_of_period: Option<MovieRef>,

    // Global-only (None for per-user)
    pub most_active_user: Option<UserRef>,
    pub most_watched_movie_global: Option<MovieRef>,
    pub total_users_active: Option<u32>,

    // Asset references for renderers
    pub poster_paths: Vec<String>,
    pub top_cast_profile_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WrapUpStatus {
    Pending,
    Generating,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WrapUpRecord {
    pub id: WrapUpId,
    pub user_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: WrapUpStatus,
    pub report_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}
