use askama::Template;
use chrono::Datelike;
use application::ports::{
    ActivityFeedPageData, HtmlPageContext, HtmlRenderer, LoginPageData,
    NewReviewPageData, ProfilePageData, RegisterPageData, UsersPageData,
};
use domain::models::{
    DiaryEntry, FeedEntry, MonthActivity, UserStats, UserSummary, UserTrends,
    collections::Paginated,
};

#[derive(Template)]
#[template(path = "diary.html")]
struct DiaryTemplate<'a> {
    entries: &'a [DiaryEntry],
    current_offset: u32,
    limit: u32,
    has_more: bool,
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    error: Option<&'a str>,
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate<'a> {
    error: Option<&'a str>,
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "new_review.html")]
struct NewReviewTemplate<'a> {
    error: Option<&'a str>,
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "activity_feed.html")]
struct ActivityFeedTemplate<'a> {
    entries: &'a [FeedEntry],
    current_offset: u32,
    limit: u32,
    has_more: bool,
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "users.html")]
struct UsersTemplate<'a> {
    users: &'a [UserSummary],
    ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate<'a> {
    ctx: &'a HtmlPageContext,
    profile_display_name: String,
    stats: &'a UserStats,
    view: &'a str,
    entries: Option<&'a Paginated<DiaryEntry>>,
    current_offset: u32,
    has_more: bool,
    limit: u32,
    history: Option<&'a Vec<MonthActivity>>,
    trends: Option<&'a UserTrends>,
    heatmap: Vec<HeatmapCell>,
}

struct HeatmapCell {
    month_label: String,
    count: i64,
    bg_style: String,
}

#[allow(dead_code)]
fn relative_time(dt: chrono::NaiveDateTime) -> String {
    let now = chrono::Utc::now().naive_utc();
    let diff = now.signed_duration_since(dt);
    if diff.num_seconds() <= 0 { return "just now".to_string(); }
    let minutes = diff.num_minutes();
    let hours = diff.num_hours();
    let days = diff.num_days();
    if minutes < 1 { return "just now".to_string(); }
    if minutes < 60 { return format!("{} min ago", minutes); }
    if hours < 24 { return format!("{} h ago", hours); }
    if days == 1 { return "yesterday".to_string(); }
    if days < 30 { return format!("{} days ago", days); }
    dt.format("%b %-d, %Y").to_string()
}

fn build_heatmap(history: &[MonthActivity]) -> Vec<HeatmapCell> {
    let current_year = chrono::Utc::now().year();
    let count_for = |m: &str| -> i64 {
        history.iter().find(|a| a.year_month == format!("{}-{}", current_year, m))
            .map(|a| a.count)
            .unwrap_or(0)
    };
    let months = [
        ("01", "Jan"), ("02", "Feb"), ("03", "Mar"), ("04", "Apr"),
        ("05", "May"), ("06", "Jun"), ("07", "Jul"), ("08", "Aug"),
        ("09", "Sep"), ("10", "Oct"), ("11", "Nov"), ("12", "Dec"),
    ];
    let counts: Vec<i64> = months.iter().map(|(m, _)| count_for(m)).collect();
    let max = counts.iter().copied().max().unwrap_or(0).max(1);
    months.iter().zip(counts.iter()).map(|((_, label), &count)| {
        let alpha = if count == 0 { 0.05 } else { 0.15 + 0.75 * (count as f64 / max as f64) };
        HeatmapCell {
            month_label: label.to_string(),
            count,
            bg_style: format!("background: rgba(74, 158, 255, {:.2})", alpha),
        }
    }).collect()
}

pub struct AskamaHtmlRenderer;

impl AskamaHtmlRenderer {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRenderer for AskamaHtmlRenderer {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>, ctx: HtmlPageContext) -> Result<String, String> {
        let has_more = (data.offset + data.limit) < data.total_count as u32;
        DiaryTemplate {
            entries: &data.items,
            current_offset: data.offset,
            limit: data.limit,
            has_more,
            ctx: &ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_login_page(&self, data: LoginPageData<'_>) -> Result<String, String> {
        LoginTemplate {
            error: data.error,
            ctx: &data.ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_register_page(&self, data: RegisterPageData<'_>) -> Result<String, String> {
        RegisterTemplate {
            error: data.error,
            ctx: &data.ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_new_review_page(&self, data: NewReviewPageData<'_>) -> Result<String, String> {
        NewReviewTemplate {
            error: data.error,
            ctx: &data.ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_activity_feed_page(&self, data: ActivityFeedPageData) -> Result<String, String> {
        ActivityFeedTemplate {
            entries: &data.entries.items,
            current_offset: data.current_offset,
            limit: data.limit,
            has_more: data.has_more,
            ctx: &data.ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_users_page(&self, data: UsersPageData) -> Result<String, String> {
        UsersTemplate {
            users: &data.users,
            ctx: &data.ctx,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_profile_page(&self, data: ProfilePageData) -> Result<String, String> {
        let heatmap = data.history.as_deref()
            .map(|h| build_heatmap(h))
            .unwrap_or_default();
        let profile_display_name = data.profile_user_email
            .split('@').next().unwrap_or(&data.profile_user_email).to_string();
        ProfileTemplate {
            ctx: &data.ctx,
            profile_display_name,
            stats: &data.stats,
            view: &data.view,
            entries: data.entries.as_ref(),
            current_offset: data.current_offset,
            has_more: data.has_more,
            limit: data.limit,
            history: data.history.as_ref(),
            trends: data.trends.as_ref(),
            heatmap,
        }
        .render()
        .map_err(|e| e.to_string())
    }
}
