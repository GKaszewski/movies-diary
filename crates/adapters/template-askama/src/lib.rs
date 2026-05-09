use askama::Template;
use chrono::Datelike;
use application::ports::{
    ActivityFeedPageData, FollowersPageData, FollowingPageData, HtmlPageContext, HtmlRenderer,
    LoginPageData, NewReviewPageData, ProfilePageData, RegisterPageData, UsersPageData,
};
use domain::models::{
    DiaryEntry, FeedEntry, MonthActivity, MonthlyRating, ReviewSource, UserStats,
    UserTrends, collections::Paginated,
};

struct PageItem {
    number: u32,
    is_current: bool,
    is_ellipsis: bool,
}

fn build_page_items(total_pages: u32, current_page: u32) -> Vec<PageItem> {
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
            items.push(PageItem { number: 0, is_current: false, is_ellipsis: true });
        }
        items.push(PageItem { number: p, is_current: p == current_page, is_ellipsis: false });
    }
    items
}

#[derive(Template)]
#[template(path = "diary.html")]
struct DiaryTemplate<'a> {
    entries: &'a [DiaryEntry],
    current_offset: u32,
    limit: u32,
    has_more: bool,
    ctx: &'a HtmlPageContext,
    page_items: Vec<PageItem>,
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
    page_items: Vec<PageItem>,
}

struct UserSummaryView {
    user_id: uuid::Uuid,
    display_name: String,
    initial: char,
    avg_rating_display: String,
    total_movies: i64,
}

#[derive(Template)]
#[template(path = "users.html")]
struct UsersTemplate<'a> {
    users: Vec<UserSummaryView>,
    ctx: &'a HtmlPageContext,
}

struct MonthlyRatingRow<'a> {
    rating: &'a MonthlyRating,
    bar_height_px: i64,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate<'a> {
    ctx: &'a HtmlPageContext,
    profile_display_name: String,
    profile_user_id: uuid::Uuid,
    stats: &'a UserStats,
    avg_rating_display: String,
    favorite_director_display: String,
    most_active_month_display: String,
    view: &'a str,
    entries: Option<&'a Paginated<DiaryEntry>>,
    current_offset: u32,
    has_more: bool,
    limit: u32,
    history: Option<&'a Vec<MonthActivity>>,
    trends: Option<&'a UserTrends>,
    monthly_rating_rows: Vec<MonthlyRatingRow<'a>>,
    heatmap: Vec<HeatmapCell>,
    page_items: Vec<PageItem>,
    is_own_profile: bool,
    error: Option<String>,
    following_count: usize,
    followers_count: usize,
    pending_followers: Vec<RemoteActorData>,
}

struct RemoteActorData {
    handle: String,
    display_name: Option<String>,
    url: String,
}

#[derive(Template)]
#[template(path = "following.html")]
struct FollowingTemplate {
    ctx: HtmlPageContext,
    user_id: uuid::Uuid,
    actors: Vec<RemoteActorData>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "followers.html")]
struct FollowersTemplate {
    ctx: HtmlPageContext,
    user_id: uuid::Uuid,
    actors: Vec<RemoteActorData>,
    error: Option<String>,
}

struct HeatmapCell {
    month_label: String,
    count: i64,
    alpha: f64,
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
            alpha,
        }
    }).collect()
}

fn bar_height_px(avg_rating: f64) -> i64 {
    (avg_rating / 5.0 * 60.0) as i64
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
        let (total_pages, current_page) = if data.limit > 0 {
            let tp = ((data.total_count + data.limit as u64 - 1) / data.limit as u64) as u32;
            (tp, data.offset / data.limit)
        } else {
            (0, 0)
        };
        DiaryTemplate {
            entries: &data.items,
            current_offset: data.offset,
            limit: data.limit,
            has_more,
            ctx: &ctx,
            page_items: build_page_items(total_pages, current_page),
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
        let limit = data.limit;
        let total_pages = if limit > 0 {
            ((data.entries.total_count + limit as u64 - 1) / limit as u64) as u32
        } else { 0 };
        let current_page = if limit > 0 { data.current_offset / limit } else { 0 };
        ActivityFeedTemplate {
            entries: &data.entries.items,
            current_offset: data.current_offset,
            limit,
            has_more: data.has_more,
            ctx: &data.ctx,
            page_items: build_page_items(total_pages, current_page),
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_users_page(&self, data: UsersPageData) -> Result<String, String> {
        let users: Vec<UserSummaryView> = data.users.iter().map(|u| {
            let email = u.email();
            let display_name = email.split('@').next().unwrap_or(email).to_string();
            let initial = display_name.chars().next().unwrap_or('?').to_ascii_uppercase();
            let avg_rating_display = u.avg_rating
                .map(|r| format!("{:.1}", r))
                .unwrap_or_else(|| "—".to_string());
            UserSummaryView {
                user_id: u.user_id.value(),
                display_name,
                initial,
                avg_rating_display,
                total_movies: u.total_movies,
            }
        }).collect();
        UsersTemplate {
            users,
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
        let monthly_rating_rows: Vec<MonthlyRatingRow<'_>> = data.trends.as_ref()
            .map(|t| t.monthly_ratings.iter().map(|r| MonthlyRatingRow {
                bar_height_px: bar_height_px(r.avg_rating),
                rating: r,
            }).collect())
            .unwrap_or_default();
        let total_pages = data.entries.as_ref()
            .map(|e| if e.limit > 0 { ((e.total_count + e.limit as u64 - 1) / e.limit as u64) as u32 } else { 0 })
            .unwrap_or(0);
        let current_page = if data.limit > 0 { data.current_offset / data.limit } else { 0 };
        let avg_rating_display = data.stats.avg_rating
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "—".to_string());
        let favorite_director_display = data.stats.favorite_director
            .as_deref()
            .unwrap_or("—")
            .to_string();
        let most_active_month_display = data.stats.most_active_month
            .as_deref()
            .unwrap_or("—")
            .to_string();
        ProfileTemplate {
            ctx: &data.ctx,
            profile_display_name,
            profile_user_id: data.profile_user_id,
            stats: &data.stats,
            avg_rating_display,
            favorite_director_display,
            most_active_month_display,
            view: &data.view,
            entries: data.entries.as_ref(),
            current_offset: data.current_offset,
            has_more: data.has_more,
            limit: data.limit,
            history: data.history.as_ref(),
            trends: data.trends.as_ref(),
            monthly_rating_rows,
            heatmap,
            page_items: build_page_items(total_pages, current_page),
            is_own_profile: data.is_own_profile,
            error: data.error,
            following_count: data.following_count,
            followers_count: data.followers_count,
            pending_followers: data.pending_followers.into_iter().map(|a| RemoteActorData {
                handle: a.handle,
                url: a.url,
                display_name: a.display_name,
            }).collect(),
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_following_page(&self, data: FollowingPageData) -> Result<String, String> {
        FollowingTemplate {
            ctx: data.ctx,
            user_id: data.user_id,
            actors: data.actors.into_iter().map(|a| RemoteActorData {
                handle: a.handle,
                display_name: a.display_name,
                url: a.url,
            }).collect(),
            error: data.error,
        }
        .render()
        .map_err(|e| e.to_string())
    }

    fn render_followers_page(&self, data: FollowersPageData) -> Result<String, String> {
        FollowersTemplate {
            ctx: data.ctx,
            user_id: data.user_id,
            actors: data.actors.into_iter().map(|a| RemoteActorData {
                handle: a.handle,
                display_name: a.display_name,
                url: a.url,
            }).collect(),
            error: data.error,
        }
        .render()
        .map_err(|e| e.to_string())
    }
}
