use api_types::HtmlPageContext;
use askama::Template;

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
