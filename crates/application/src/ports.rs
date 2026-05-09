use uuid::Uuid;

use domain::models::{DiaryEntry, FeedEntry, MonthActivity, UserStats, UserSummary, UserTrends, collections::Paginated};

pub struct RemoteActorView {
    pub handle: String,
    pub display_name: Option<String>,
    pub url: String,
}

pub struct HtmlPageContext {
    pub user_email: Option<String>,
    pub user_id: Option<Uuid>,
    pub register_enabled: bool,
    pub rss_url: String,
    pub page_title: String,
    pub canonical_url: String,
}

impl HtmlPageContext {
    pub fn is_current_user(&self, id: Uuid) -> bool {
        self.user_id == Some(id)
    }
}

pub struct LoginPageData<'a> {
    pub ctx: HtmlPageContext,
    pub error: Option<&'a str>,
}

pub struct RegisterPageData<'a> {
    pub ctx: HtmlPageContext,
    pub error: Option<&'a str>,
}

pub struct NewReviewPageData<'a> {
    pub ctx: HtmlPageContext,
    pub error: Option<&'a str>,
}

pub struct ActivityFeedPageData {
    pub ctx: HtmlPageContext,
    pub entries: Paginated<FeedEntry>,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
}

pub struct UsersPageData {
    pub ctx: HtmlPageContext,
    pub users: Vec<UserSummary>,
}

pub struct ProfilePageData {
    pub ctx: HtmlPageContext,
    pub profile_user_id: Uuid,
    pub profile_user_email: String,
    pub stats: UserStats,
    pub view: String,
    pub entries: Option<Paginated<DiaryEntry>>,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub history: Option<Vec<MonthActivity>>,
    pub trends: Option<UserTrends>,
    pub is_own_profile: bool,
    pub error: Option<String>,
    pub following_count: usize,
    pub followers_count: usize,
    pub pending_followers: Vec<RemoteActorView>,
}

pub struct FollowingPageData {
    pub ctx: HtmlPageContext,
    pub user_id: Uuid,
    pub actors: Vec<RemoteActorView>,
    pub error: Option<String>,
}

pub struct FollowersPageData {
    pub ctx: HtmlPageContext,
    pub user_id: Uuid,
    pub actors: Vec<RemoteActorView>,
    pub error: Option<String>,
}

pub trait HtmlRenderer: Send + Sync {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>, ctx: HtmlPageContext) -> Result<String, String>;
    fn render_login_page(&self, data: LoginPageData<'_>) -> Result<String, String>;
    fn render_register_page(&self, data: RegisterPageData<'_>) -> Result<String, String>;
    fn render_new_review_page(&self, data: NewReviewPageData<'_>) -> Result<String, String>;
    fn render_activity_feed_page(&self, data: ActivityFeedPageData) -> Result<String, String>;
    fn render_users_page(&self, data: UsersPageData) -> Result<String, String>;
    fn render_profile_page(&self, data: ProfilePageData) -> Result<String, String>;
    fn render_following_page(&self, data: FollowingPageData) -> Result<String, String>;
    fn render_followers_page(&self, data: FollowersPageData) -> Result<String, String>;
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String>;
}
