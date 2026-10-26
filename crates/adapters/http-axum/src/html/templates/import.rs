use api_types::HtmlPageContext;
use askama::Template;

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
