use domain::{
    errors::DomainError,
    models::{
        AnnotatedRow, FieldMapping,
        import::{DomainField, ImportRow, RowResult, Transform},
    },
};
use serde::{Deserialize, Serialize};

// ── serde mirror structs (match the JSON format from the old importer types) ──

#[derive(Serialize, Deserialize, Default)]
pub(super) struct ParsedFileJson {
    pub(super) columns: Vec<String>,
    pub(super) rows: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub(super) enum DomainFieldJson {
    Title,
    ReleaseYear,
    Director,
    Rating,
    WatchedAt,
    Comment,
    ExternalMetadataId,
}

#[derive(Serialize, Deserialize)]
pub(super) enum TransformJson {
    RatingScale(f64),
    DateFormat(String),
    Identity,
}

#[derive(Serialize, Deserialize)]
pub(super) struct FieldMappingJson {
    source_column: String,
    domain_field: DomainFieldJson,
    transform: TransformJson,
}

#[derive(Serialize, Deserialize, Default)]
pub(super) struct ImportRowJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    director: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    watched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_metadata_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(super) enum RowResultJson {
    Valid(ImportRowJson),
    Invalid {
        errors: Vec<String>,
        raw: Vec<(String, String)>,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct AnnotatedRowJson {
    result: RowResultJson,
    is_duplicate: bool,
}

// ── conversion helpers ──

pub(super) fn domain_field_to_json(f: &DomainField) -> DomainFieldJson {
    match f {
        DomainField::Title => DomainFieldJson::Title,
        DomainField::ReleaseYear => DomainFieldJson::ReleaseYear,
        DomainField::Director => DomainFieldJson::Director,
        DomainField::Rating => DomainFieldJson::Rating,
        DomainField::WatchedAt => DomainFieldJson::WatchedAt,
        DomainField::Comment => DomainFieldJson::Comment,
        DomainField::ExternalMetadataId => DomainFieldJson::ExternalMetadataId,
    }
}

pub(super) fn domain_field_from_json(j: DomainFieldJson) -> DomainField {
    match j {
        DomainFieldJson::Title => DomainField::Title,
        DomainFieldJson::ReleaseYear => DomainField::ReleaseYear,
        DomainFieldJson::Director => DomainField::Director,
        DomainFieldJson::Rating => DomainField::Rating,
        DomainFieldJson::WatchedAt => DomainField::WatchedAt,
        DomainFieldJson::Comment => DomainField::Comment,
        DomainFieldJson::ExternalMetadataId => DomainField::ExternalMetadataId,
    }
}

pub(super) fn transform_to_json(t: &Transform) -> TransformJson {
    match t {
        Transform::RatingScale(f) => TransformJson::RatingScale(*f),
        Transform::DateFormat(s) => TransformJson::DateFormat(s.clone()),
        Transform::Identity => TransformJson::Identity,
    }
}

pub(super) fn transform_from_json(j: TransformJson) -> Transform {
    match j {
        TransformJson::RatingScale(f) => Transform::RatingScale(f),
        TransformJson::DateFormat(s) => Transform::DateFormat(s),
        TransformJson::Identity => Transform::Identity,
    }
}

pub(super) fn mapping_to_json(m: &FieldMapping) -> FieldMappingJson {
    FieldMappingJson {
        source_column: m.source_column.clone(),
        domain_field: domain_field_to_json(&m.domain_field),
        transform: transform_to_json(&m.transform),
    }
}

pub(super) fn mapping_from_json(j: FieldMappingJson) -> FieldMapping {
    FieldMapping {
        source_column: j.source_column,
        domain_field: domain_field_from_json(j.domain_field),
        transform: transform_from_json(j.transform),
    }
}

pub(super) fn import_row_to_json(r: &ImportRow) -> ImportRowJson {
    ImportRowJson {
        title: r.title.clone(),
        release_year: r.release_year.clone(),
        director: r.director.clone(),
        rating: r.rating.clone(),
        watched_at: r.watched_at.clone(),
        comment: r.comment.clone(),
        external_metadata_id: r.external_metadata_id.clone(),
    }
}

pub(super) fn import_row_from_json(j: ImportRowJson) -> ImportRow {
    ImportRow {
        title: j.title,
        release_year: j.release_year,
        director: j.director,
        rating: j.rating,
        watched_at: j.watched_at,
        comment: j.comment,
        external_metadata_id: j.external_metadata_id,
    }
}

pub(super) fn annotated_to_json(a: &AnnotatedRow) -> AnnotatedRowJson {
    AnnotatedRowJson {
        result: match &a.result {
            RowResult::Valid(row) => RowResultJson::Valid(import_row_to_json(row)),
            RowResult::Invalid { errors, raw } => RowResultJson::Invalid {
                errors: errors.clone(),
                raw: raw.clone(),
            },
        },
        is_duplicate: a.is_duplicate,
    }
}

pub(super) fn annotated_from_json(j: AnnotatedRowJson) -> AnnotatedRow {
    AnnotatedRow {
        result: match j.result {
            RowResultJson::Valid(row) => RowResult::Valid(import_row_from_json(row)),
            RowResultJson::Invalid { errors, raw } => RowResult::Invalid { errors, raw },
        },
        is_duplicate: j.is_duplicate,
    }
}

pub(super) fn ser<T: Serialize>(v: &T) -> Result<String, DomainError> {
    serde_json::to_string(v).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

pub(super) fn de<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, DomainError> {
    serde_json::from_str(s).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}
