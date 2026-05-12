use super::*;
use domain::models::{DomainField, FieldMapping, ParsedFile, RowResult, Transform};

fn sample_file() -> ParsedFile {
    ParsedFile {
        columns: vec!["Name".into(), "Stars".into(), "Date".into()],
        rows: vec![
            vec!["Inception".into(), "10".into(), "2024-01-15".into()],
            vec!["Dune".into(), "8".into(), "2024-02-20".into()],
            vec!["".into(), "3".into(), "2024-03-01".into()], // missing title → invalid
        ],
    }
}

fn full_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping { source_column: "Name".into(), domain_field: DomainField::Title, transform: Transform::Identity },
        FieldMapping { source_column: "Stars".into(), domain_field: DomainField::Rating, transform: Transform::RatingScale(0.5) },
        FieldMapping { source_column: "Date".into(), domain_field: DomainField::WatchedAt, transform: Transform::Identity },
    ]
}

#[test]
fn maps_valid_rows() {
    let results = apply_mapping(&sample_file(), &full_mappings());
    assert_eq!(results.len(), 3);
    // First two rows are valid
    assert!(matches!(results[0].result, RowResult::Valid(_)));
    assert!(matches!(results[1].result, RowResult::Valid(_)));
    // is_duplicate defaults to false
    assert!(!results[0].is_duplicate);
}

#[test]
fn applies_rating_scale_transform() {
    let results = apply_mapping(&sample_file(), &full_mappings());
    if let RowResult::Valid(row) = &results[0].result {
        // 10 * 0.5 = 5
        assert_eq!(row.rating.as_deref(), Some("5"));
    } else {
        panic!("expected Valid");
    }
}

#[test]
fn marks_missing_required_fields_invalid() {
    let results = apply_mapping(&sample_file(), &full_mappings());
    // Row 2 has empty title
    assert!(matches!(results[2].result, RowResult::Invalid { .. }));
}

#[test]
fn ignores_unmapped_columns() {
    let mappings = vec![
        FieldMapping { source_column: "Name".into(), domain_field: DomainField::Title, transform: Transform::Identity },
    ];
    let file = ParsedFile {
        columns: vec!["Name".into(), "Extra".into()],
        rows: vec![vec!["Inception".into(), "ignored".into()]],
    };
    let results = apply_mapping(&file, &mappings);
    assert_eq!(results.len(), 1);
    // Missing rating and watched_at → invalid
    assert!(matches!(results[0].result, RowResult::Invalid { .. }));
}

#[test]
fn nonexistent_source_column_skipped() {
    let mappings = vec![
        FieldMapping { source_column: "DoesNotExist".into(), domain_field: DomainField::Title, transform: Transform::Identity },
    ];
    let file = ParsedFile {
        columns: vec!["Name".into()],
        rows: vec![vec!["Inception".into()]],
    };
    let results = apply_mapping(&file, &mappings);
    // Column not found → field not set → invalid (missing title, rating, watched_at)
    assert!(matches!(results[0].result, RowResult::Invalid { .. }));
}

#[test]
fn collects_all_errors_not_just_first() {
    let mappings = vec![
        FieldMapping { source_column: "Name".into(), domain_field: DomainField::Title, transform: Transform::Identity },
        FieldMapping { source_column: "Stars".into(), domain_field: DomainField::Rating, transform: Transform::RatingScale(0.5) },
        // no watched_at mapping
    ];
    let file = ParsedFile {
        columns: vec!["Name".into(), "Stars".into()],
        rows: vec![vec!["Inception".into(), "notanumber".into()]],
    };
    let results = apply_mapping(&file, &mappings);
    if let RowResult::Invalid { errors, .. } = &results[0].result {
        assert!(errors.iter().any(|e| e.contains("not a number")), "expected rating error, got: {:?}", errors);
        assert!(errors.iter().any(|e| e.contains("watched_at")), "expected watched_at error, got: {:?}", errors);
    } else {
        panic!("expected Invalid");
    }
}

#[test]
fn non_numeric_rating_produces_error_in_row() {
    let mappings = vec![
        FieldMapping { source_column: "Name".into(), domain_field: DomainField::Title, transform: Transform::Identity },
        FieldMapping { source_column: "Stars".into(), domain_field: DomainField::Rating, transform: Transform::RatingScale(0.5) },
        FieldMapping { source_column: "Date".into(), domain_field: DomainField::WatchedAt, transform: Transform::Identity },
    ];
    let file = ParsedFile {
        columns: vec!["Name".into(), "Stars".into(), "Date".into()],
        rows: vec![vec!["Inception".into(), "five".into(), "2024-01-15".into()]],
    };
    let results = apply_mapping(&file, &mappings);
    assert!(matches!(results[0].result, RowResult::Invalid { .. }));
}
