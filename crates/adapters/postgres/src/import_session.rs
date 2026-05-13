use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::{
        AnnotatedRow, FieldMapping, ImportSession, ParsedFile,
        import::{DomainField, ImportRow, RowResult, Transform},
    },
    ports::ImportSessionRepository,
    value_objects::{ImportSessionId, UserId},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── serde mirror structs ──

#[derive(Serialize, Deserialize, Default)]
struct ParsedFileJson {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
enum DomainFieldJson {
    Title,
    ReleaseYear,
    Director,
    Rating,
    WatchedAt,
    Comment,
    ExternalMetadataId,
}

#[derive(Serialize, Deserialize)]
enum TransformJson {
    RatingScale(f64),
    DateFormat(String),
    Identity,
}

#[derive(Serialize, Deserialize)]
struct FieldMappingJson {
    source_column: String,
    domain_field: DomainFieldJson,
    transform: TransformJson,
}

#[derive(Serialize, Deserialize, Default)]
struct ImportRowJson {
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
enum RowResultJson {
    Valid(ImportRowJson),
    Invalid {
        errors: Vec<String>,
        raw: Vec<(String, String)>,
    },
}

#[derive(Serialize, Deserialize)]
struct AnnotatedRowJson {
    result: RowResultJson,
    is_duplicate: bool,
}

// ── conversion helpers ──

fn domain_field_to_json(f: &DomainField) -> DomainFieldJson {
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

fn domain_field_from_json(j: DomainFieldJson) -> DomainField {
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

fn transform_to_json(t: &Transform) -> TransformJson {
    match t {
        Transform::RatingScale(f) => TransformJson::RatingScale(*f),
        Transform::DateFormat(s) => TransformJson::DateFormat(s.clone()),
        Transform::Identity => TransformJson::Identity,
    }
}

fn transform_from_json(j: TransformJson) -> Transform {
    match j {
        TransformJson::RatingScale(f) => Transform::RatingScale(f),
        TransformJson::DateFormat(s) => Transform::DateFormat(s),
        TransformJson::Identity => Transform::Identity,
    }
}

fn mapping_to_json(m: &FieldMapping) -> FieldMappingJson {
    FieldMappingJson {
        source_column: m.source_column.clone(),
        domain_field: domain_field_to_json(&m.domain_field),
        transform: transform_to_json(&m.transform),
    }
}

fn mapping_from_json(j: FieldMappingJson) -> FieldMapping {
    FieldMapping {
        source_column: j.source_column,
        domain_field: domain_field_from_json(j.domain_field),
        transform: transform_from_json(j.transform),
    }
}

fn import_row_to_json(r: &ImportRow) -> ImportRowJson {
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

fn import_row_from_json(j: ImportRowJson) -> ImportRow {
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

fn annotated_to_json(a: &AnnotatedRow) -> AnnotatedRowJson {
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

fn annotated_from_json(j: AnnotatedRowJson) -> AnnotatedRow {
    AnnotatedRow {
        result: match j.result {
            RowResultJson::Valid(row) => RowResult::Valid(import_row_from_json(row)),
            RowResultJson::Invalid { errors, raw } => RowResult::Invalid { errors, raw },
        },
        is_duplicate: j.is_duplicate,
    }
}

fn ser<T: Serialize>(v: &T) -> Result<String, DomainError> {
    serde_json::to_string(v).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

fn de<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, DomainError> {
    serde_json::from_str(s).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

// ── repository ──

pub struct PostgresImportSessionRepository {
    pool: PgPool,
}

impl PostgresImportSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("DB error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }

    fn serialize_session(
        s: &ImportSession,
    ) -> Result<(String, Option<String>, Option<String>), DomainError> {
        let parsed = s
            .parsed_file
            .as_ref()
            .map(|f| {
                ser(&ParsedFileJson {
                    columns: f.columns.clone(),
                    rows: f.rows.clone(),
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mappings = s
            .field_mappings
            .as_ref()
            .map(|ms| ser(&ms.iter().map(mapping_to_json).collect::<Vec<_>>()))
            .transpose()?;
        let results = s
            .row_results
            .as_ref()
            .map(|rs| ser(&rs.iter().map(annotated_to_json).collect::<Vec<_>>()))
            .transpose()?;
        Ok((parsed, mappings, results))
    }

    fn deserialize_session(
        id: String,
        user_id: String,
        parsed_data: String,
        field_mappings: Option<String>,
        row_results: Option<String>,
        created_at: NaiveDateTime,
        expires_at: NaiveDateTime,
    ) -> Result<ImportSession, DomainError> {
        let parsed_file = if parsed_data.is_empty() {
            None
        } else {
            let j: ParsedFileJson = de(&parsed_data)?;
            Some(ParsedFile {
                columns: j.columns,
                rows: j.rows,
            })
        };
        let field_mappings = field_mappings
            .as_deref()
            .map(|s| -> Result<Vec<FieldMapping>, DomainError> {
                let js: Vec<FieldMappingJson> = de(s)?;
                Ok(js.into_iter().map(mapping_from_json).collect())
            })
            .transpose()?;
        let row_results = row_results
            .as_deref()
            .map(|s| -> Result<Vec<AnnotatedRow>, DomainError> {
                let js: Vec<AnnotatedRowJson> = de(s)?;
                Ok(js.into_iter().map(annotated_from_json).collect())
            })
            .transpose()?;
        Ok(ImportSession {
            id: ImportSessionId::from_uuid(
                id.parse::<uuid::Uuid>()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
            ),
            user_id: UserId::from_uuid(
                user_id
                    .parse::<uuid::Uuid>()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
            ),
            parsed_file,
            field_mappings,
            row_results,
            created_at,
            expires_at,
        })
    }
}

#[async_trait]
impl ImportSessionRepository for PostgresImportSessionRepository {
    async fn create(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let user_id = s.user_id.value().to_string();
        let (parsed_data, field_mappings, row_results) = Self::serialize_session(s)?;
        sqlx::query(
            "INSERT INTO import_sessions (id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id).bind(&user_id).bind(&parsed_data)
        .bind(&field_mappings).bind(&row_results)
        .bind(s.created_at).bind(s.expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            user_id: String,
            parsed_data: String,
            field_mappings: Option<String>,
            row_results: Option<String>,
            created_at: NaiveDateTime,
            expires_at: NaiveDateTime,
        }

        let row = sqlx::query_as::<_, Row>(
            "SELECT id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at
             FROM import_sessions WHERE id = $1 AND user_id = $2",
        )
        .bind(&id_str)
        .bind(&uid_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        row.map(|r| {
            Self::deserialize_session(
                r.id,
                r.user_id,
                r.parsed_data,
                r.field_mappings,
                r.row_results,
                r.created_at,
                r.expires_at,
            )
        })
        .transpose()
    }

    async fn update(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let (_, field_mappings, row_results) = Self::serialize_session(s)?;
        sqlx::query(
            "UPDATE import_sessions SET field_mappings = $1, row_results = $2 WHERE id = $3",
        )
        .bind(&field_mappings)
        .bind(&row_results)
        .bind(&id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query("DELETE FROM import_sessions WHERE id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM import_sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(result.rows_affected())
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query("DELETE FROM import_sessions WHERE user_id = $1 AND expires_at < NOW()")
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }
}
