use super::{PageItem, RemoteActorData, filters};
use api_types::HtmlPageContext;
use askama::Template;
use chrono::Datelike;
use domain::models::{
    DiaryEntry, MonthActivity, MonthlyRating, UserStats, UserTrends, collections::Paginated,
};

pub struct RemoteActorDisplay {
    pub handle: String,
    pub display_name: String,
    pub initial: char,
    pub url: String,
}

pub struct UserSummaryView {
    pub user_id: uuid::Uuid,
    pub display_name: String,
    pub initial: char,
    pub avg_rating_display: String,
    pub total_movies: i64,
    pub avatar_url: Option<String>,
}

#[derive(Template)]
#[template(path = "users.html")]
pub struct UsersTemplate<'a> {
    pub users: Vec<UserSummaryView>,
    pub ctx: &'a HtmlPageContext,
    pub remote_actors: Vec<RemoteActorDisplay>,
}

pub struct MonthlyRatingRow<'a> {
    pub rating: &'a MonthlyRating,
    pub bar_height_px: i64,
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub profile_display_name: String,
    pub profile_user_id: uuid::Uuid,
    pub stats: &'a UserStats,
    pub avg_rating_display: String,
    pub favorite_director_display: String,
    pub most_active_month_display: String,
    pub view: &'a str,
    pub entries: Option<&'a Paginated<DiaryEntry>>,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub history: Option<&'a Vec<MonthActivity>>,
    pub trends: Option<&'a UserTrends>,
    pub monthly_rating_rows: Vec<MonthlyRatingRow<'a>>,
    pub heatmap: Vec<HeatmapCell>,
    pub page_items: Vec<PageItem>,
    pub is_own_profile: bool,
    pub is_service_account: bool,
    pub error: Option<String>,
    pub following_count: usize,
    pub followers_count: usize,
    pub pending_followers: Vec<RemoteActorData>,
    pub sort_by: String,
    pub search: String,
    pub goals: Vec<GoalViewData>,
}

pub struct GoalViewData {
    pub year: u16,
    pub target_count: u32,
    pub current_count: u32,
    pub percentage: f64,
    pub is_complete: bool,
}

impl<'a> ProfileTemplate<'a> {
    pub fn filter_qs(&self) -> String {
        let mut parts = vec![
            format!("view={}", self.view),
            format!("sort_by={}", self.sort_by),
        ];
        if !self.search.is_empty() {
            let encoded = self
                .search
                .replace(' ', "+")
                .replace('#', "%23")
                .replace('&', "%26")
                .replace('=', "%3D");
            parts.push(format!("search={}", encoded));
        }
        format!("&{}", parts.join("&"))
    }
}

#[derive(Template)]
#[template(path = "embed_profile.html")]
pub struct EmbedProfileTemplate<'a> {
    pub profile_display_name: String,
    pub profile_user_id: uuid::Uuid,
    pub profile_url: String,
    pub stats: &'a UserStats,
    pub avg_rating_display: String,
    pub favorite_director_display: String,
    pub most_active_month_display: String,
    pub view: &'a str,
    pub entries: Option<&'a Paginated<DiaryEntry>>,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub history: Option<&'a Vec<MonthActivity>>,
    pub trends: Option<&'a UserTrends>,
    pub monthly_rating_rows: Vec<MonthlyRatingRow<'a>>,
    pub heatmap: Vec<HeatmapCell>,
    pub page_items: Vec<PageItem>,
    pub sort_by: String,
}

impl<'a> EmbedProfileTemplate<'a> {
    pub fn filter_qs(&self) -> String {
        let parts = [
            format!("view={}", self.view),
            format!("sort_by={}", self.sort_by),
            "embed=true".to_string(),
        ];
        format!("&{}", parts.join("&"))
    }
}

pub struct HeatmapCell {
    pub month_label: String,
    pub count: i64,
    pub alpha: f64,
}

pub fn build_heatmap(history: &[MonthActivity]) -> Vec<HeatmapCell> {
    let current_year = chrono::Utc::now().year();
    let count_for = |m: &str| -> i64 {
        history
            .iter()
            .find(|a| a.year_month == format!("{}-{}", current_year, m))
            .map(|a| a.count)
            .unwrap_or(0)
    };
    let months = [
        ("01", "Jan"),
        ("02", "Feb"),
        ("03", "Mar"),
        ("04", "Apr"),
        ("05", "May"),
        ("06", "Jun"),
        ("07", "Jul"),
        ("08", "Aug"),
        ("09", "Sep"),
        ("10", "Oct"),
        ("11", "Nov"),
        ("12", "Dec"),
    ];
    let counts: Vec<i64> = months.iter().map(|(m, _)| count_for(m)).collect();
    let max = counts.iter().copied().max().unwrap_or(0).max(1);
    months
        .iter()
        .zip(counts.iter())
        .map(|((_, label), &count)| {
            let alpha = if count == 0 {
                0.05
            } else {
                0.15 + 0.75 * (count as f64 / max as f64)
            };
            HeatmapCell {
                month_label: label.to_string(),
                count,
                alpha,
            }
        })
        .collect()
}

pub fn bar_height_px(avg_rating: f64) -> i64 {
    (avg_rating / 5.0 * 60.0) as i64
}

#[derive(Template)]
#[template(path = "profile_settings.html")]
pub struct ProfileSettingsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub bio: Option<&'a str>,
    pub avatar_url: Option<&'a str>,
    pub banner_url: Option<&'a str>,
    pub also_known_as: Option<&'a str>,
    pub profile_fields: &'a [(String, String)],
    pub saved: bool,
    pub error: Option<&'a str>,
    pub embed_url: String,
}
