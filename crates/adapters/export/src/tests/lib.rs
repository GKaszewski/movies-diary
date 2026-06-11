use super::ExportAdapter;
use domain::{
    models::{DiaryEntry, ExportFormat, Movie, Review},
    ports::DiaryExporter,
    value_objects::{ExternalMetadataId, MovieTitle, Rating, ReleaseYear},
};

async fn collect_stream(
    stream: futures::stream::BoxStream<'static, Result<bytes::Bytes, domain::errors::DomainError>>,
) -> Vec<u8> {
    use futures::StreamExt;
    let mut out = Vec::new();
    futures::pin_mut!(stream);
    while let Some(chunk) = stream.next().await {
        out.extend_from_slice(&chunk.unwrap());
    }
    out
}

fn entry_stream(
    entries: Vec<domain::models::DiaryEntry>,
) -> futures::stream::BoxStream<'static, Result<domain::models::DiaryEntry, domain::errors::DomainError>>
{
    Box::pin(futures::stream::iter(entries.into_iter().map(Ok)))
}

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
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![entry]), ExportFormat::Csv)).await;
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.starts_with("title,year,director,rating,comment,watched_at,external_metadata_id\n")
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
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![entry]), ExportFormat::Csv)).await;
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("\"Tár, A Film\""));
}

#[tokio::test]
async fn json_is_valid_array() {
    let adapter = ExportAdapter;
    let entry = make_entry("Dune", 2021, Some("Denis Villeneuve"), 5, None);
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![entry]), ExportFormat::Json)).await;
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
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![entry]), ExportFormat::Json)).await;
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(arr[0]["external_metadata_id"], "tt0078748");

    let bytes = collect_stream(
        adapter.stream_entries(
            entry_stream(vec![make_entry_full("Alien", 1979, None, 5, None, Some("tt0078748"))]),
            ExportFormat::Csv,
        ),
    )
    .await;
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("tt0078748"));
}

#[tokio::test]
async fn empty_entries_returns_csv_header_only() {
    let adapter = ExportAdapter;
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![]), ExportFormat::Csv)).await;
    let text = String::from_utf8(bytes).unwrap();
    assert_eq!(
        text,
        "title,year,director,rating,comment,watched_at,external_metadata_id\n"
    );
}

#[tokio::test]
async fn empty_json_is_valid_empty_array() {
    let adapter = ExportAdapter;
    let bytes =
        collect_stream(adapter.stream_entries(entry_stream(vec![]), ExportFormat::Json)).await;
    let arr: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert!(arr.is_empty());
}
