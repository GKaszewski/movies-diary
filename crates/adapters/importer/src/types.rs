use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedFile {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DomainField {
    Title,
    ReleaseYear,
    Director,
    Rating,
    WatchedAt,
    Comment,
    ExternalMetadataId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
    RatingScale(f64),
    DateFormat(String),
    Identity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub source_column: String,
    pub domain_field: DomainField,
    pub transform: Transform,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportRow {
    pub title: Option<String>,
    pub release_year: Option<String>,
    pub director: Option<String>,
    pub rating: Option<String>,
    pub watched_at: Option<String>,
    pub comment: Option<String>,
    pub external_metadata_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RowResult {
    Valid(ImportRow),
    Invalid { errors: Vec<String>, raw: Vec<(String, String)> },
}

/// Wraps a RowResult with a duplicate flag so this information persists when
/// serialised as JSON into the import_sessions.row_results DB column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedRow {
    pub result: RowResult,
    pub is_duplicate: bool,
}
