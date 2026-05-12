use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, ExportFormat},
    ports::DiaryExporter,
};

pub struct ExportAdapter;

#[async_trait]
impl DiaryExporter for ExportAdapter {
    async fn serialize_entries(
        &self,
        entries: &[DiaryEntry],
        format: ExportFormat,
    ) -> Result<Vec<u8>, DomainError> {
        match format {
            ExportFormat::Csv => serialize_csv(entries),
            ExportFormat::Json => serialize_json(entries),
        }
    }
}

fn serialize_csv(entries: &[DiaryEntry]) -> Result<Vec<u8>, DomainError> {
    let mut out =
        String::from("title,year,director,rating,comment,watched_at,external_metadata_id\n");
    for e in entries {
        let title = csv_escape(e.movie().title().value());
        let year = e.movie().release_year().value();
        let director = e.movie().director().map(csv_escape).unwrap_or_default();
        let rating = e.review().rating().value();
        let comment = e
            .review()
            .comment()
            .map(|c| csv_escape(c.value()))
            .unwrap_or_default();
        let watched_at = e.review().watched_at().format("%Y-%m-%d");
        let ext_id = e
            .movie()
            .external_metadata_id()
            .map(|id| id.value().to_string())
            .unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            title, year, director, rating, comment, watched_at, ext_id
        ));
    }
    Ok(out.into_bytes())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn serialize_json(entries: &[DiaryEntry]) -> Result<Vec<u8>, DomainError> {
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "title": e.movie().title().value(),
                "year": e.movie().release_year().value(),
                "director": e.movie().director(),
                "rating": e.review().rating().value(),
                "comment": e.review().comment().map(|c| c.value()),
                "watched_at": e.review().watched_at().format("%Y-%m-%d").to_string(),
                "external_metadata_id": e.movie().external_metadata_id().map(|id| id.value()),
            })
        })
        .collect();
    serde_json::to_vec_pretty(&arr).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
