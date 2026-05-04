use askama::Template;
use application::ports::{
    HtmlPageContext, HtmlRenderer, LoginPageData, NewReviewPageData, RegisterPageData,
};
use domain::models::{DiaryEntry, collections::Paginated};

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
}
