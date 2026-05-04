use domain::models::{DiaryEntry, collections::Paginated};

pub struct HtmlPageContext {
    pub user_email: Option<String>,
    pub register_enabled: bool,
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

pub trait HtmlRenderer: Send + Sync {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>, ctx: HtmlPageContext) -> Result<String, String>;
    fn render_login_page(&self, data: LoginPageData<'_>) -> Result<String, String>;
    fn render_register_page(&self, data: RegisterPageData<'_>) -> Result<String, String>;
    fn render_new_review_page(&self, data: NewReviewPageData<'_>) -> Result<String, String>;
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry]) -> Result<String, String>;
}
