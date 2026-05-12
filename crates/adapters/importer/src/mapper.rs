use domain::models::{
    AnnotatedRow, DomainField, FieldMapping, ImportRow, ParsedFile, RowResult, Transform,
};

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
#[path = "tests/mapper.rs"]
mod tests;
