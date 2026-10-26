use crate::extractors::{Body, PathId, Upload};
use crate::support::str_to_domain_field;
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::{
    ApplyMappingRequest, ConfirmRequest, ImportPreviewResponse, PreviewRowDto,
    SessionCreatedResponse, SessionStateResponse,
};
use application::import::{
    apply_mapping as apply_import_mapping, apply_profile_and_map,
    commands::{
        ApplyImportMappingCommand, ApplyProfileAndMapCommand, CreateImportSessionCommand,
        ExecuteImportCommand,
    },
    create_session as create_import_session, execute as execute_import, get_preview_stage,
    get_session_state,
};
use axum::{extract::State, response::IntoResponse};
use domain::errors::DomainError;
use domain::models::{
    AnnotatedRow, FieldMapping, FileFormat,
    import::{DomainField, Transform},
};
use domain::value_objects::ImportSessionId;

#[utoipa::path(
    post, path = "/api/v1/import/sessions",
    request_body(content_type = "multipart/form-data", description = "file (binary) + format (csv|json|xlsx)"),
    responses(
        (status = 200, body = SessionCreatedResponse),
        (status = 400, description = "No file provided", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 422, description = "Parse error", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_session(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Upload(mut multipart): Upload,
) -> Result<impl IntoResponse, ApiError> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut format_str = "csv".to_string();
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Ok(b) = field.bytes().await {
                    file_bytes = Some(b.to_vec());
                }
            }
            Some("format") => {
                if let Ok(t) = field.text().await {
                    format_str = t;
                }
            }
            _ => {}
        }
    }
    let bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return Err(DomainError::ValidationError("no file".into()).into()),
    };
    let format = match format_str.as_str() {
        "json" => FileFormat::Json,
        "xlsx" => FileFormat::Xlsx,
        _ => FileFormat::Csv,
    };
    let r = create_import_session::execute(
        &state.app_ctx.deps.import.create_session,
        CreateImportSessionCommand {
            user_id: user_id.value(),
            bytes,
            format,
        },
    )
    .await?;
    Ok(axum::Json(SessionCreatedResponse {
        session_id: r.session_id.value().to_string(),
        columns: r.columns,
        sample_rows: r.sample_rows,
    }))
}

#[utoipa::path(
    get, path = "/api/v1/import/sessions/{id}",
    params(("id" = String, Path, description = "Import session UUID")),
    responses(
        (status = 200, body = SessionStateResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Session not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_get_session(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(session_id_str): PathId<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;
    let state_data = get_session_state::execute(
        &state.app_ctx.deps.import.get_session_state,
        session_id,
        user_id.value(),
    )
    .await?;
    Ok(axum::Json(SessionStateResponse {
        session_id: session_id_str,
        columns: state_data.columns,
        has_mappings: state_data.has_mappings,
        row_count: state_data.row_count,
    }))
}

#[utoipa::path(
    put, path = "/api/v1/import/sessions/{id}/mapping",
    params(("id" = String, Path, description = "Import session UUID")),
    request_body = ApplyMappingRequest,
    responses(
        (status = 200, description = "Mapping applied", body = inline(serde_json::Value)),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 422, description = "Mapping error", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_put_mapping(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(session_id_str): PathId<String>,
    Body(body): Body<ApplyMappingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;
    let mappings: Vec<FieldMapping> = body
        .mappings
        .into_iter()
        .filter_map(|m| {
            let domain_field = str_to_domain_field(&m.domain_field)?;
            let transform = if domain_field == DomainField::Rating {
                Transform::RatingScale(m.rating_scale.unwrap_or(1.0))
            } else if domain_field == DomainField::WatchedAt {
                m.date_format
                    .map(Transform::DateFormat)
                    .unwrap_or(Transform::Identity)
            } else {
                Transform::Identity
            };
            Some(FieldMapping {
                source_column: m.source_column,
                domain_field,
                transform,
            })
        })
        .collect();

    let rows = apply_import_mapping::execute(
        &state.app_ctx.deps.import.apply_mapping,
        ApplyImportMappingCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            mappings,
        },
    )
    .await?;
    Ok(axum::Json(serde_json::json!({"row_count": rows.len()})))
}

pub async fn api_get_preview(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(session_id_str): PathId<String>,
) -> Result<axum::Json<ImportPreviewResponse>, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;

    let stage = get_preview_stage::execute(
        &state.app_ctx.deps.import.get_preview_stage,
        session_id,
        user_id.value(),
    )
    .await?;

    // Not-yet-mapped sessions render as an empty preview here (200, `rows: []`),
    // matching this endpoint's behavior before the stage-gate moved into
    // `get_preview_stage` — unlike the HTML handler, this API route never
    // redirected on the same condition, so its response shape is unchanged.
    let annotated: Vec<AnnotatedRow> = match stage {
        get_preview_stage::PreviewStage::Ready(preview) => preview.rows,
        get_preview_stage::PreviewStage::NotYetMapped => Vec::new(),
    };
    let rows = annotated
        .iter()
        .enumerate()
        .map(|(i, a)| {
            use domain::models::import::RowResult;
            match &a.result {
                RowResult::Valid(row) if a.is_duplicate => {
                    PreviewRowDto::Duplicate(api_types::PreviewRowData {
                        index: i,
                        title: row.title.clone(),
                        release_year: row.release_year.clone(),
                        director: row.director.clone(),
                        rating: row.rating.clone(),
                        watched_at: row.watched_at.clone(),
                        comment: row.comment.clone(),
                    })
                }
                RowResult::Valid(row) => PreviewRowDto::Valid(api_types::PreviewRowData {
                    index: i,
                    title: row.title.clone(),
                    release_year: row.release_year.clone(),
                    director: row.director.clone(),
                    rating: row.rating.clone(),
                    watched_at: row.watched_at.clone(),
                    comment: row.comment.clone(),
                }),
                RowResult::Invalid { errors, .. } => PreviewRowDto::Invalid {
                    index: i,
                    errors: errors.clone(),
                },
            }
        })
        .collect();

    Ok(axum::Json(ImportPreviewResponse { rows }))
}

#[utoipa::path(
    post, path = "/api/v1/import/sessions/{id}/confirm",
    params(("id" = String, Path, description = "Import session UUID")),
    request_body = ConfirmRequest,
    responses(
        (status = 200, description = "Import summary", body = inline(serde_json::Value)),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Session not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_confirm(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(session_id_str): PathId<String>,
    Body(body): Body<ConfirmRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;
    let s = execute_import::execute(
        &state.app_ctx.deps.import.execute_import,
        ExecuteImportCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            confirmed_indices: body.confirmed_indices,
        },
    )
    .await?;
    Ok(axum::Json(serde_json::json!({
        "imported": s.imported,
        "skipped_duplicates": s.skipped_duplicates,
        "failed": s.failed.iter().map(|(i, e)| serde_json::json!({"index": i, "error": e})).collect::<Vec<_>>(),
    })))
}

#[utoipa::path(
    put, path = "/api/v1/import/sessions/{id}/profile/{profile_id}",
    params(
        ("id" = String, Path, description = "Import session UUID"),
        ("profile_id" = String, Path, description = "Import profile UUID"),
    ),
    responses(
        (status = 200, description = "Profile applied and mapping regenerated", body = inline(serde_json::Value)),
        (status = 400, description = "Invalid ID", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Session or profile not found", body = api_types::ErrorResponse),
        (status = 422, description = "Mapping error", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_apply_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId((session_id_str, profile_id_str)): PathId<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;
    let profile_id = profile_id_str
        .parse::<uuid::Uuid>()
        .map_err(|_| DomainError::ValidationError("invalid profile id".into()))?;

    let rows = apply_profile_and_map::execute(
        &state.app_ctx.deps.import.apply_profile_and_map,
        ApplyProfileAndMapCommand {
            user_id: user_id.value(),
            session_id,
            profile_id,
        },
    )
    .await?;
    Ok(axum::Json(serde_json::json!({"row_count": rows.len()})))
}
