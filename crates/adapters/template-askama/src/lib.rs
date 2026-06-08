pub use askama;
use askama::Template;

use application::ports::HtmlPageContext;
use chrono::Datelike;
use domain::models::{
    DiaryEntry, FeedEntry, MonthActivity, MonthlyRating, ReviewSource, UserStats, UserTrends,
    collections::Paginated,
};

mod filters {
    #[askama::filter_fn]
    pub fn poster_src<T: std::fmt::Display>(
        path: T,
        _env: &dyn askama::Values,
    ) -> askama::Result<String> {
        let p = path.to_string();
        if p.starts_with("http://") || p.starts_with("https://") {
            Ok(p)
        } else {
            Ok(format!("/images/{}", p))
        }
    }
}

pub struct PageItem {
    pub number: u32,
    pub is_current: bool,
    pub is_ellipsis: bool,
}

pub fn build_page_items(total_pages: u32, current_page: u32) -> Vec<PageItem> {
    if total_pages <= 1 {
        return vec![];
    }
    let mut set = std::collections::BTreeSet::new();
    set.insert(0u32);
    set.insert(total_pages - 1);
    let start = current_page.saturating_sub(2);
    let end = (current_page + 2).min(total_pages - 1);
    for p in start..=end {
        set.insert(p);
    }
    let pages: Vec<u32> = set.into_iter().collect();
    let mut items = Vec::new();
    for (i, &p) in pages.iter().enumerate() {
        if i > 0 && p > pages[i - 1] + 1 {
            items.push(PageItem {
                number: 0,
                is_current: false,
                is_ellipsis: true,
            });
        }
        items.push(PageItem {
            number: p,
            is_current: p == current_page,
            is_ellipsis: false,
        });
    }
    items
}

#[derive(Template)]
#[template(path = "diary.html")]
pub struct DiaryTemplate<'a> {
    pub entries: &'a [DiaryEntry],
    pub current_offset: u32,
    pub limit: u32,
    pub has_more: bool,
    pub ctx: &'a HtmlPageContext,
    pub page_items: Vec<PageItem>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    pub error: Option<&'a str>,
    pub ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate<'a> {
    pub error: Option<&'a str>,
    pub ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "new_review.html")]
pub struct NewReviewTemplate<'a> {
    pub error: Option<&'a str>,
    pub ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "activity_feed.html")]
pub struct ActivityFeedTemplate<'a> {
    pub entries: &'a [FeedEntry],
    pub current_offset: u32,
    pub limit: u32,
    pub has_more: bool,
    pub ctx: &'a HtmlPageContext,
    pub page_items: Vec<PageItem>,
    pub filter: String,
    pub sort_by: String,
    pub search: String,
}

#[derive(Template)]
#[template(path = "movie_detail.html")]
pub struct MovieDetailTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub movie: &'a domain::models::Movie,
    pub stats: &'a domain::models::MovieStats,
    pub profile: Option<&'a domain::models::MovieProfile>,
    pub reviews: &'a [domain::models::FeedEntry],
    pub on_watchlist: bool,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub histogram_max: u64,
}

#[derive(Template)]
#[template(path = "watchlist.html")]
pub struct WatchlistTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub owner_id: uuid::Uuid,
    pub display_entries: &'a [application::ports::WatchlistDisplayEntry],
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub is_owner: bool,
    pub error: Option<String>,
}

impl<'a> ActivityFeedTemplate<'a> {
    pub fn filter_qs(&self) -> String {
        let mut parts = vec![
            format!("filter={}", self.filter),
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

pub struct RemoteActorData {
    pub handle: String,
    pub display_name: Option<String>,
    pub url: String,
    pub avatar_url: Option<String>,
}

#[derive(Template)]
#[template(path = "following.html")]
pub struct FollowingTemplate {
    pub ctx: HtmlPageContext,
    pub user_id: uuid::Uuid,
    pub actors: Vec<RemoteActorData>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "followers.html")]
pub struct FollowersTemplate {
    pub ctx: HtmlPageContext,
    pub user_id: uuid::Uuid,
    pub actors: Vec<RemoteActorData>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "blocked_domains.html")]
pub struct BlockedDomainsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub domains: &'a [BlockedDomainEntry],
}

#[derive(Template)]
#[template(path = "blocked_actors.html")]
pub struct BlockedActorsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub actors: &'a [BlockedActorEntry],
}

pub struct BlockedDomainEntry {
    pub domain: String,
    pub reason: Option<String>,
    pub blocked_at: String,
}

pub struct BlockedActorEntry {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
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
    pub embed_url: String,
}

#[derive(Template)]
#[template(path = "integrations.html")]
pub struct IntegrationsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub tokens: &'a [WebhookTokenView],
    pub webhook_base_url: &'a str,
    pub new_token: Option<&'a str>,
}

pub struct WebhookTokenView {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Template)]
#[template(path = "watch_queue.html")]
pub struct WatchQueueTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub entries: &'a [WatchQueueDisplayEntry],
    pub error: Option<&'a str>,
}

pub struct WatchQueueDisplayEntry {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub source: String,
    pub watched_at: String,
    pub movie_url: Option<String>,
}

#[derive(Template)]
#[template(path = "import_upload.html")]
pub struct ImportUploadTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub profiles: &'a [ImportProfileView],
    pub error: Option<&'a str>,
}

pub struct ImportProfileView {
    pub id: String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "import_mapping.html")]
pub struct ImportMappingTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub session_id: &'a str,
    pub columns: &'a [String],
    pub sample_rows: &'a [Vec<String>],
    pub domain_fields: &'a [(&'static str, &'static str)],
    pub error: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "import_preview.html")]
pub struct ImportPreviewTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub session_id: &'a str,
    pub columns: &'a [String],
    pub rows: &'a [ImportPreviewRow],
}

pub struct ImportPreviewRow {
    pub index: usize,
    pub status: ImportRowStatus,
    pub cells: Vec<String>,
}

pub enum ImportRowStatus {
    Valid,
    Duplicate,
    Invalid(String),
}

#[derive(Template)]
#[template(path = "wrapup.html")]
pub struct WrapUpPageTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub report: &'a domain::models::wrapup::WrapUpReport,
    pub year_label: String,
    pub watch_time_display: String,
    pub rating_max: u32,
    pub genre_max: u32,
    pub rating_pcts: [f64; 5],
    pub genre_pcts: Vec<f64>,
    pub video_url: Option<String>,
}
