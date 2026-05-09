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
mod tests {
    use super::ExportAdapter;
    use domain::{
        models::{DiaryEntry, ExportFormat, Movie, Review},
        ports::DiaryExporter,
        value_objects::{ExternalMetadataId, MovieTitle, Rating, ReleaseYear},
    };

    fn make_entry(
        title: &str,
        year: u16,
        director: Option<&str>,
        rating: u8,
        comment: Option<&str>,
    ) -> DiaryEntry {
        make_entry_full(title, year, director, rating, comment, None)
    }

    fn make_entry_full(
        title: &str,
        year: u16,
        director: Option<&str>,
        rating: u8,
        comment: Option<&str>,
        external_id: Option<&str>,
    ) -> DiaryEntry {
        let movie = Movie::new(
            external_id.map(|id| ExternalMetadataId::new(id.to_string()).unwrap()),
            MovieTitle::new(title.to_string()).unwrap(),
            ReleaseYear::new(year).unwrap(),
            director.map(str::to_string),
            None,
        );
        let user_id = domain::value_objects::UserId::from_uuid(uuid::Uuid::new_v4());
        let review = Review::new(
            movie.id().clone(),
            user_id,
            Rating::new(rating).unwrap(),
            comment.map(|c| domain::value_objects::Comment::new(c.to_string()).unwrap()),
            chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        )
        .unwrap();
        DiaryEntry::new(movie, review)
    }

    #[tokio::test]
    async fn csv_has_header_and_one_row() {
        let adapter = ExportAdapter;
        let entry = make_entry(
            "Inception",
            2010,
            Some("Christopher Nolan"),
            5,
            Some("great"),
        );
        let bytes = adapter
            .serialize_entries(&[entry], ExportFormat::Csv)
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            text.starts_with(
                "title,year,director,rating,comment,watched_at,external_metadata_id\n"
            )
        );
        assert!(text.contains("Inception"));
        assert!(text.contains("2010"));
        assert!(text.contains("Christopher Nolan"));
        assert!(text.contains("5"));
        assert!(text.contains("great"));
        assert!(text.contains("2024-03-15"));
    }

    #[tokio::test]
    async fn csv_escapes_commas_in_title() {
        let adapter = ExportAdapter;
        let entry = make_entry("Tár, A Film", 2022, None, 4, None);
        let bytes = adapter
            .serialize_entries(&[entry], ExportFormat::Csv)
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"Tár, A Film\""));
    }

    #[tokio::test]
    async fn json_is_valid_array() {
        let adapter = ExportAdapter;
        let entry = make_entry("Dune", 2021, Some("Denis Villeneuve"), 5, None);
        let bytes = adapter
            .serialize_entries(&[entry], ExportFormat::Json)
            .await
            .unwrap();
        let arr: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Dune");
        assert_eq!(arr[0]["year"], 2021);
        assert_eq!(arr[0]["rating"], 5);
        assert_eq!(arr[0]["comment"], serde_json::Value::Null);
        assert_eq!(arr[0]["external_metadata_id"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn external_metadata_id_included_when_present() {
        let adapter = ExportAdapter;
        let entry = make_entry_full("Alien", 1979, None, 5, None, Some("tt0078748"));
        let bytes = adapter
            .serialize_entries(&[entry], ExportFormat::Json)
            .await
            .unwrap();
        let arr: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(arr[0]["external_metadata_id"], "tt0078748");

        let bytes = adapter
            .serialize_entries(
                &[make_entry_full(
                    "Alien",
                    1979,
                    None,
                    5,
                    None,
                    Some("tt0078748"),
                )],
                ExportFormat::Csv,
            )
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("tt0078748"));
    }

    #[tokio::test]
    async fn empty_entries_returns_csv_header_only() {
        let adapter = ExportAdapter;
        let bytes = adapter
            .serialize_entries(&[], ExportFormat::Csv)
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text,
            "title,year,director,rating,comment,watched_at,external_metadata_id\n"
        );
    }
}
