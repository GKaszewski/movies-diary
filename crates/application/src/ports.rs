use uuid::Uuid;

use domain::models::{
    DiaryEntry, FeedEntry, MonthActivity, Movie, MovieStats, UserStats, UserSummary, UserTrends,
    collections::Paginated,
};

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
    pub csrf_token: String,
    pub page_rss_url: Option<String>,
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
    pub filter: String,
    pub sort_by: String,
    pub search: String,
}

pub struct UsersPageData {
    pub ctx: HtmlPageContext,
    pub users: Vec<UserSummary>,
    pub remote_actors: Vec<RemoteActorView>,
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
    pub sort_by: String,
    pub search: String,
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

pub struct MovieDetailPageData {
    pub ctx: HtmlPageContext,
    pub movie: Movie,
    pub stats: MovieStats,
    pub reviews: Paginated<FeedEntry>,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub histogram_max: u64,
}

pub struct ImportUploadPageData {
    pub ctx: HtmlPageContext,
    pub profiles: Vec<ImportProfileView>,
    pub error: Option<String>,
}

pub struct ImportProfileView {
    pub id: String,
    pub name: String,
}

pub struct ImportMappingPageData {
    pub ctx: HtmlPageContext,
    pub session_id: String,
    pub columns: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub domain_fields: Vec<(&'static str, &'static str)>,
    pub error: Option<String>,
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

pub struct ImportPreviewPageData {
    pub ctx: HtmlPageContext,
    pub session_id: String,
    pub columns: Vec<String>,
    pub rows: Vec<ImportPreviewRow>,
}

pub struct ProfileSettingsPageData {
    pub ctx: HtmlPageContext,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub saved: bool,
}

pub trait HtmlRenderer: Send + Sync {
    fn render_diary_page(
        &self,
        data: &Paginated<DiaryEntry>,
        ctx: HtmlPageContext,
    ) -> Result<String, String>;
    fn render_login_page(&self, data: LoginPageData<'_>) -> Result<String, String>;
    fn render_register_page(&self, data: RegisterPageData<'_>) -> Result<String, String>;
    fn render_new_review_page(&self, data: NewReviewPageData<'_>) -> Result<String, String>;
    fn render_activity_feed_page(&self, data: ActivityFeedPageData) -> Result<String, String>;
    fn render_users_page(&self, data: UsersPageData) -> Result<String, String>;
    fn render_profile_page(&self, data: ProfilePageData) -> Result<String, String>;
    fn render_following_page(&self, data: FollowingPageData) -> Result<String, String>;
    fn render_followers_page(&self, data: FollowersPageData) -> Result<String, String>;
    fn render_movie_detail_page(&self, data: MovieDetailPageData) -> Result<String, String>;
    fn render_import_upload_page(&self, data: ImportUploadPageData) -> Result<String, String>;
    fn render_import_mapping_page(&self, data: ImportMappingPageData) -> Result<String, String>;
    fn render_import_preview_page(&self, data: ImportPreviewPageData) -> Result<String, String>;
    fn render_profile_settings_page(
        &self,
        data: ProfileSettingsPageData,
    ) -> Result<String, String>;
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String>;
}
