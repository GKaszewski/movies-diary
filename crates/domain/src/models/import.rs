use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct ParsedFile {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainField {
    Title,
    ReleaseYear,
    Director,
    Rating,
    WatchedAt,
    Comment,
    ExternalMetadataId,
}

#[derive(Debug, Clone)]
pub enum Transform {
    RatingScale(f64),
    DateFormat(String),
    Identity,
}

#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub source_column: String,
    pub domain_field: DomainField,
    pub transform: Transform,
}

#[derive(Debug, Clone, Default)]
pub struct ImportRow {
    pub title: Option<String>,
    pub release_year: Option<String>,
    pub director: Option<String>,
    pub rating: Option<String>,
    pub watched_at: Option<String>,
    pub comment: Option<String>,
    pub external_metadata_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RowResult {
    Valid(ImportRow),
    Invalid { errors: Vec<String>, raw: Vec<(String, String)> },
}

#[derive(Debug, Clone)]
pub struct AnnotatedRow {
    pub result: RowResult,
    pub is_duplicate: bool,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("CSV parse error: {0}")]
    Csv(String),
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("XLSX parse error: {0}")]
    Xlsx(String),
    #[error("Empty file")]
    Empty,
    #[error("Missing header row")]
    NoHeader,
}

pub enum FileFormat {
    Csv,
    Json,
    Xlsx,
}
