use crate::types::{AnnotatedRow, DomainField, FieldMapping, ImportRow, ParsedFile, RowResult, Transform};

pub fn apply_mapping(file: &ParsedFile, mappings: &[FieldMapping]) -> Vec<AnnotatedRow> {
    file.rows.iter().map(|row| {
        let result = map_row(row, &file.columns, mappings);
        AnnotatedRow { result, is_duplicate: false }
    }).collect()
}

fn map_row(row: &[String], columns: &[String], mappings: &[FieldMapping]) -> RowResult {
    let mut import_row = ImportRow::default();
    let mut errors = Vec::new();

    for mapping in mappings {
        let Some(col_idx) = columns.iter().position(|c| c == &mapping.source_column) else {
            continue;
        };
        let raw_value = row.get(col_idx).map(|s| s.as_str()).unwrap_or("").trim();
        if raw_value.is_empty() {
            continue;
        }
        if let Some(value) = apply_transform(raw_value, &mapping.transform, &mut errors) {
            set_field(&mut import_row, &mapping.domain_field, value);
        }
    }

    if import_row.title.is_none() && import_row.external_metadata_id.is_none() {
        errors.push("missing required field: title or external_metadata_id".into());
    }
    if import_row.rating.is_none() {
        errors.push("missing required field: rating".into());
    }
    if import_row.watched_at.is_none() {
        errors.push("missing required field: watched_at".into());
    }

    if errors.is_empty() {
        RowResult::Valid(import_row)
    } else {
        let raw = columns.iter()
            .zip(row.iter())
            .map(|(c, v)| (c.clone(), v.clone()))
            .collect();
        RowResult::Invalid { errors, raw }
    }
}

fn apply_transform(value: &str, transform: &Transform, errors: &mut Vec<String>) -> Option<String> {
    match transform {
        Transform::Identity => Some(value.to_string()),
        Transform::DateFormat(_) => Some(value.to_string()),
        Transform::RatingScale(factor) => {
            match value.parse::<f64>() {
                Ok(n) => Some((n * factor).round().to_string()),
                Err(_) => {
                    errors.push(format!("rating '{}' is not a number", value));
                    None
                }
            }
        }
    }
}

fn set_field(row: &mut ImportRow, field: &DomainField, value: String) {
    match field {
        DomainField::Title => row.title = Some(value),
        DomainField::ReleaseYear => row.release_year = Some(value),
        DomainField::Director => row.director = Some(value),
        DomainField::Rating => row.rating = Some(value),
        DomainField::WatchedAt => row.watched_at = Some(value),
        DomainField::Comment => row.comment = Some(value),
        DomainField::ExternalMetadataId => row.external_metadata_id = Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DomainField, FieldMapping, ParsedFile, RowResult, Transform};

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
}
