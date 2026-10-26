use domain::{
    errors::DomainError,
    models::{
        FieldMapping,
        import::{DomainField, Transform},
    },
};
use serde::{Deserialize, Serialize};

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

pub(super) fn mapping_to_json(m: &FieldMapping) -> FieldMappingJson {
    FieldMappingJson {
        source_column: m.source_column.clone(),
        domain_field: match &m.domain_field {
            DomainField::Title => DomainFieldJson::Title,
            DomainField::ReleaseYear => DomainFieldJson::ReleaseYear,
            DomainField::Director => DomainFieldJson::Director,
            DomainField::Rating => DomainFieldJson::Rating,
            DomainField::WatchedAt => DomainFieldJson::WatchedAt,
            DomainField::Comment => DomainFieldJson::Comment,
            DomainField::ExternalMetadataId => DomainFieldJson::ExternalMetadataId,
        },
        transform: match &m.transform {
            Transform::RatingScale(f) => TransformJson::RatingScale(*f),
            Transform::DateFormat(s) => TransformJson::DateFormat(s.clone()),
            Transform::Identity => TransformJson::Identity,
        },
    }
}

pub(super) fn mapping_from_json(j: FieldMappingJson) -> FieldMapping {
    FieldMapping {
        source_column: j.source_column,
        domain_field: match j.domain_field {
            DomainFieldJson::Title => DomainField::Title,
            DomainFieldJson::ReleaseYear => DomainField::ReleaseYear,
            DomainFieldJson::Director => DomainField::Director,
            DomainFieldJson::Rating => DomainField::Rating,
            DomainFieldJson::WatchedAt => DomainField::WatchedAt,
            DomainFieldJson::Comment => DomainField::Comment,
            DomainFieldJson::ExternalMetadataId => DomainField::ExternalMetadataId,
        },
        transform: match j.transform {
            TransformJson::RatingScale(f) => Transform::RatingScale(f),
            TransformJson::DateFormat(s) => Transform::DateFormat(s),
            TransformJson::Identity => Transform::Identity,
        },
    }
}

pub(super) fn serialize_mappings(ms: &[FieldMapping]) -> Result<String, DomainError> {
    serde_json::to_string(&ms.iter().map(mapping_to_json).collect::<Vec<_>>())
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

pub(super) fn deserialize_mappings(s: &str) -> Result<Vec<FieldMapping>, DomainError> {
    let js: Vec<FieldMappingJson> =
        serde_json::from_str(s).map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    Ok(js.into_iter().map(mapping_from_json).collect())
}
