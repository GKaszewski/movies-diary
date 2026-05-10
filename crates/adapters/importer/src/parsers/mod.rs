mod csv;
mod json;
#[cfg(feature = "xlsx")]
mod xlsx;

pub use csv::parse_csv;
pub use json::parse_json;
#[cfg(feature = "xlsx")]
pub use xlsx::parse_xlsx;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parses_headers_and_rows() {
        let data = b"title,rating,watched_at\nInception,5,2024-01-01\nDune,4,2024-02-15\n";
        let file = parse_csv(data).unwrap();
        assert_eq!(file.columns, vec!["title", "rating", "watched_at"]);
        assert_eq!(file.rows.len(), 2);
        assert_eq!(file.rows[0], vec!["Inception", "5", "2024-01-01"]);
    }

    #[test]
    fn csv_rejects_empty() {
        assert!(parse_csv(b"").is_err());
    }

    #[test]
    fn tsv_parses_correctly() {
        let data = b"title\trating\nInception\t5\n";
        let file = parse_csv(data).unwrap();
        assert_eq!(file.columns, vec!["title", "rating"]);
        assert_eq!(file.rows[0], vec!["Inception", "5"]);
    }

    #[test]
    fn json_array_of_objects() {
        let data = br#"[{"title":"Inception","rating":"5"},{"title":"Dune","rating":"4"}]"#;
        let file = parse_json(data).unwrap();
        assert_eq!(file.columns.len(), 2);
        assert!(file.columns.contains(&"title".to_string()));
        assert_eq!(file.rows.len(), 2);
    }

    #[test]
    fn json_empty_array_errors() {
        assert!(parse_json(b"[]").is_err());
    }
}
