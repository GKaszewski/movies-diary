#[derive(Debug, thiserror::Error)]
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
