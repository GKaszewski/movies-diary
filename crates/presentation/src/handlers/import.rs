use api_types::{
    ApplyMappingRequest, ConfirmRequest, ImportPreviewResponse, PreviewRowDto, SaveProfileRequest,
    SessionCreatedResponse, SessionStateResponse,
};
use axum::{
    Extension, Form,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::render::render_page;
use application::import::{
    apply_mapping as apply_import_mapping,
    commands::{
        ApplyImportMappingCommand, CreateImportSessionCommand, DeleteImportProfileCommand,
        ExecuteImportCommand, SaveImportProfileCommand,
    },
    create_session as create_import_session, delete_profile as delete_import_profile,
    execute as execute_import, list_profiles as list_import_profiles,
    save_profile as save_import_profile,
};
use domain::models::{
    AnnotatedRow, FieldMapping, FileFormat,
    import::{DomainField, Transform},
};
use domain::value_objects::ImportSessionId;
use template_askama::{
    ImportMappingTemplate, ImportPreviewRow, ImportPreviewTemplate, ImportProfileView,
    ImportUploadTemplate,
};

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, RequiredCookieUser},
    state::AppState,
};

fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

fn str_to_domain_field(field: &str) -> Option<DomainField> {
    match field {
        "title" => Some(DomainField::Title),
        "release_year" => Some(DomainField::ReleaseYear),
        "director" => Some(DomainField::Director),
        "rating" => Some(DomainField::Rating),
        "watched_at" => Some(DomainField::WatchedAt),
        "comment" => Some(DomainField::Comment),
        "external_metadata_id" => Some(DomainField::ExternalMetadataId),
        _ => None,
    }
}

fn parse_mapping_form(form: &HashMap<String, String>) -> Vec<FieldMapping> {
    let mut mappings = Vec::new();
    let mut i = 0usize;
    loop {
        if i > 64 {
            break;
        }
        let col_key = format!("mapping_{i}_col");
        let Some(col) = form.get(&col_key).cloned() else {
            break;
        };
        let field_str = form
            .get(&format!("mapping_{i}_field"))
            .map(|s| s.as_str())
            .unwrap_or("");
        if let Some(domain_field) = str_to_domain_field(field_str) {
            let transform = if domain_field == DomainField::Rating {
                let scale: f64 = form
                    .get(&format!("mapping_{i}_scale"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1.0);
                Transform::RatingScale(scale)
            } else if domain_field == DomainField::WatchedAt {
                form.get(&format!("mapping_{i}_datefmt"))
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .map(Transform::DateFormat)
                    .unwrap_or(Transform::Identity)
            } else {
                Transform::Identity
            };
            mappings.push(FieldMapping {
                source_column: col,
                domain_field,
                transform,
            });
        }
        i += 1;
    }
    mappings
}

use crate::mappers::import::annotated_to_preview_row;

// ── HTML wizard handlers ───────────────────────────────────────────────────

pub async fn get_import_page(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = super::html::build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    let profiles = list_import_profiles::execute(&state.app_ctx, &user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| ImportProfileView {
            id: p.id.value().to_string(),
            name: p.name,
        })
        .collect::<Vec<_>>();
    render_page(ImportUploadTemplate {
        ctx: &ctx,
        profiles: &profiles,
        error: None,
    })
}

pub async fn post_upload(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut format_str = "csv".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Ok(bytes) = field.bytes().await {
                    file_bytes = Some(bytes.to_vec());
                }
            }
            Some("format") => {
                if let Ok(text) = field.text().await {
                    format_str = text;
                }
            }
            _ => {}
        }
    }

    let bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return Redirect::to("/import?error=no+file+provided").into_response(),
    };

    let format = match format_str.as_str() {
        "json" => FileFormat::Json,
        "xlsx" => FileFormat::Xlsx,
        _ => FileFormat::Csv,
    };

    match create_import_session::execute(
        &state.app_ctx,
        CreateImportSessionCommand {
            user_id: user_id.value(),
            bytes,
            format,
        },
    )
    .await
    {
        Ok(r) => Redirect::to(&format!("/import/{}/mapping", r.session_id.value())).into_response(),
        Err(e) => {
            Redirect::to(&format!("/import?error={}", encode_error(&e.to_string()))).into_response()
        }
    }
}

pub async fn get_mapping_page(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };
    let Ok(Some(session)) = state
        .app_ctx
        .repos
        .import_session
        .get(&session_id, &user_id)
        .await
    else {
        return Redirect::to("/import").into_response();
    };
    let Some(parsed) = session.parsed_file else {
        return Redirect::to("/import").into_response();
    };

    let ctx = super::html::build_page_context(&state, Some(user_id), csrf.0).await;
    let sample_rows: Vec<Vec<String>> = parsed.rows.into_iter().take(5).collect();
    let domain_fields: Vec<(&str, &str)> = vec![
        ("title", "Title"),
        ("release_year", "Release Year"),
        ("director", "Director"),
        ("rating", "Rating"),
        ("watched_at", "Watched At"),
        ("comment", "Comment"),
        ("external_metadata_id", "External ID"),
    ];
    render_page(ImportMappingTemplate {
        ctx: &ctx,
        session_id: &session_id_str,
        columns: &parsed.columns,
        sample_rows: &sample_rows,
        domain_fields: &domain_fields,
        error: None,
    })
    .into_response()
}

pub async fn post_mapping(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let csrf_token = form.get("_csrf").map(|s| s.as_str()).unwrap_or("");
    if crate::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };
    let mappings = parse_mapping_form(&form);
    if mappings.is_empty() {
        return Redirect::to(&format!(
            "/import/{}/mapping?error=select+at+least+one+mapping",
            session_id_str
        ))
        .into_response();
    }
    match apply_import_mapping::execute(
        &state.app_ctx,
        ApplyImportMappingCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            mappings,
        },
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/import/{}/preview", session_id_str)).into_response(),
        Err(e) => Redirect::to(&format!(
            "/import/{}/mapping?error={}",
            session_id_str,
            encode_error(&e.to_string())
        ))
        .into_response(),
    }
}

pub async fn get_preview_page(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };
    let Ok(Some(session)) = state
        .app_ctx
        .repos
        .import_session
        .get(&session_id, &user_id)
        .await
    else {
        return Redirect::to("/import").into_response();
    };

    if session.row_results.is_none() {
        return Redirect::to(&format!("/import/{}/mapping", session_id_str)).into_response();
    }

    let parsed = session.parsed_file.unwrap_or_default();
    let annotated: Vec<AnnotatedRow> = session.row_results.unwrap_or_default();

    let rows: Vec<ImportPreviewRow> = annotated
        .iter()
        .enumerate()
        .map(|(i, a)| annotated_to_preview_row(i, a))
        .collect();

    let ctx = super::html::build_page_context(&state, Some(user_id), csrf.0).await;
    render_page(ImportPreviewTemplate {
        ctx: &ctx,
        session_id: &session_id_str,
        columns: &parsed.columns,
        rows: &rows,
    })
    .into_response()
}

pub async fn post_confirm(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form_entries): Form<Vec<(String, String)>>,
) -> impl IntoResponse {
    let csrf_token = form_entries
        .iter()
        .find(|(k, _)| k == "_csrf")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    if crate::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };

    // Save profile if name provided
    let profile_name = form_entries
        .iter()
        .find(|(k, _)| k == "profile_name")
        .map(|(_, v)| v.clone())
        .filter(|n| !n.trim().is_empty());
    if let Some(name) = profile_name {
        let _ = save_import_profile::execute(
            &state.app_ctx,
            SaveImportProfileCommand {
                user_id: user_id.value(),
                session_id: session_id.value(),
                name,
            },
        )
        .await;
    }

    // Collect all "confirmed" checkbox values
    let confirmed: Vec<usize> = form_entries
        .iter()
        .filter(|(k, _)| k == "confirmed")
        .filter_map(|(_, v)| v.parse::<usize>().ok())
        .collect();

    match execute_import::execute(
        &state.app_ctx,
        ExecuteImportCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            confirmed_indices: confirmed,
        },
    )
    .await
    {
        Ok(summary) => Redirect::to(&format!(
            "/import/done?imported={}&skipped={}&failed={}",
            summary.imported,
            summary.skipped_duplicates,
            summary.failed.len()
        ))
        .into_response(),
        Err(e) => {
            Redirect::to(&format!("/import?error={}", encode_error(&e.to_string()))).into_response()
        }
    }
}

pub async fn post_delete_profile(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(profile_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let csrf_token = form.get("_csrf").map(|s| s.as_str()).unwrap_or("");
    if crate::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    if let Ok(profile_id) = profile_id_str.parse::<uuid::Uuid>() {
        let _ = delete_import_profile::execute(
            &state.app_ctx,
            DeleteImportProfileCommand {
                user_id: user_id.value(),
                profile_id,
            },
        )
        .await;
    }
    Redirect::to("/import").into_response()
}

#[derive(Deserialize)]
pub struct ImportDoneParams {
    pub imported: Option<usize>,
    pub skipped: Option<usize>,
    pub failed: Option<usize>,
}

pub async fn get_import_done(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    axum::extract::Query(params): axum::extract::Query<ImportDoneParams>,
) -> impl IntoResponse {
    let _ctx = super::html::build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    let html = format!(
        r#"<!doctype html><html><body>
        <h1>Import Complete</h1>
        <p>Imported: {}</p>
        <p>Skipped duplicates: {}</p>
        <p>Failed: {}</p>
        <a href="/users/{}">Go to My Profile</a>
        </body></html>"#,
        params.imported.unwrap_or(0),
        params.skipped.unwrap_or(0),
        params.failed.unwrap_or(0),
        user_id.value(),
    );
    Html(html)
}

// ── REST API handlers ──────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/import/sessions",
    request_body(content_type = "multipart/form-data", description = "file (binary) + format (csv|json|xlsx)"),
    responses(
        (status = 200, body = SessionCreatedResponse),
        (status = 400, description = "No file provided"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Parse error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_session(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
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
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({"error": "no file"})),
            )
                .into_response();
        }
    };
    let format = match format_str.as_str() {
        "json" => FileFormat::Json,
        "xlsx" => FileFormat::Xlsx,
        _ => FileFormat::Csv,
    };
    match create_import_session::execute(
        &state.app_ctx,
        CreateImportSessionCommand {
            user_id: user_id.value(),
            bytes,
            format,
        },
    )
    .await
    {
        Ok(r) => axum::Json(SessionCreatedResponse {
            session_id: r.session_id.value().to_string(),
            columns: r.columns,
            sample_rows: r.sample_rows,
        })
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(
    get, path = "/api/v1/import/sessions/{id}",
    params(("id" = String, Path, description = "Import session UUID")),
    responses(
        (status = 200, body = SessionStateResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_get_session(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(session_id_str): Path<String>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "invalid session id"})),
        )
            .into_response();
    };
    match state
        .app_ctx
        .repos
        .import_session
        .get(&session_id, &user_id)
        .await
    {
        Ok(Some(session)) => {
            let parsed = session.parsed_file.unwrap_or_default();
            let row_count = parsed.rows.len();
            axum::Json(SessionStateResponse {
                session_id: session_id_str,
                columns: parsed.columns,
                has_mappings: session.field_mappings.is_some(),
                row_count,
            })
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({"error": "session not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(
    put, path = "/api/v1/import/sessions/{id}/mapping",
    params(("id" = String, Path, description = "Import session UUID")),
    request_body = ApplyMappingRequest,
    responses(
        (status = 200, description = "Mapping applied", body = inline(serde_json::Value)),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Mapping error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_put_mapping(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(session_id_str): Path<String>,
    axum::Json(body): axum::Json<ApplyMappingRequest>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "invalid session id"})),
        )
            .into_response();
    };
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

    match apply_import_mapping::execute(
        &state.app_ctx,
        ApplyImportMappingCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            mappings,
        },
    )
    .await
    {
        Ok(rows) => axum::Json(serde_json::json!({"row_count": rows.len()})).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn api_get_preview(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(session_id_str): Path<String>,
) -> Result<axum::Json<ImportPreviewResponse>, ApiError> {
    let session_id = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| {
            ApiError(domain::errors::DomainError::ValidationError(
                "invalid session id".into(),
            ))
        })?;

    let session = state
        .app_ctx
        .repos
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| {
            ApiError(domain::errors::DomainError::NotFound(
                "session not found".into(),
            ))
        })?;

    let annotated: Vec<AnnotatedRow> = session.row_results.unwrap_or_default();
    let rows = annotated
        .iter()
        .enumerate()
        .map(|(i, a)| {
            use domain::models::import::RowResult;
            match &a.result {
                RowResult::Valid(row) if a.is_duplicate => PreviewRowDto::Duplicate {
                    index: i,
                    title: row.title.clone(),
                    release_year: row.release_year.clone(),
                    director: row.director.clone(),
                    rating: row.rating.clone(),
                    watched_at: row.watched_at.clone(),
                    comment: row.comment.clone(),
                },
                RowResult::Valid(row) => PreviewRowDto::Valid {
                    index: i,
                    title: row.title.clone(),
                    release_year: row.release_year.clone(),
                    director: row.director.clone(),
                    rating: row.rating.clone(),
                    watched_at: row.watched_at.clone(),
                    comment: row.comment.clone(),
                },
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_confirm(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(session_id_str): Path<String>,
    axum::Json(body): axum::Json<ConfirmRequest>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "invalid session id"})),
        )
            .into_response();
    };
    match execute_import::execute(&state.app_ctx, ExecuteImportCommand { user_id: user_id.value(), session_id: session_id.value(), confirmed_indices: body.confirmed_indices }).await {
        Ok(s) => axum::Json(serde_json::json!({
            "imported": s.imported,
            "skipped_duplicates": s.skipped_duplicates,
            "failed": s.failed.iter().map(|(i, e)| serde_json::json!({"index": i, "error": e})).collect::<Vec<_>>(),
        })).into_response(),
        Err(e) => {
            let status = if matches!(e, domain::errors::DomainError::NotFound(_)) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, axum::Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/import/profiles",
    responses(
        (status = 200, description = "List of saved import profiles", body = inline(Vec<serde_json::Value>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_get_profiles(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> impl IntoResponse {
    match list_import_profiles::execute(&state.app_ctx, &user_id).await {
        Ok(profiles) => axum::Json(
            profiles
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id.value().to_string(),
                        "name": p.name,
                        "created_at": p.created_at.to_string(),
                    })
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post, path = "/api/v1/import/profiles",
    request_body = SaveProfileRequest,
    responses(
        (status = 200, description = "Profile saved", body = inline(serde_json::Value)),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Session has no mapping yet"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    axum::Json(body): axum::Json<SaveProfileRequest>,
) -> impl IntoResponse {
    let Ok(session_id) = body
        .session_id
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "invalid session id"})),
        )
            .into_response();
    };
    match save_import_profile::execute(
        &state.app_ctx,
        SaveImportProfileCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            name: body.name,
        },
    )
    .await
    {
        Ok(id) => axum::Json(serde_json::json!({"id": id.value().to_string()})).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(
    delete, path = "/api/v1/import/profiles/{id}",
    params(("id" = String, Path, description = "Import profile UUID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_delete_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(profile_id_str): Path<String>,
) -> impl IntoResponse {
    let Ok(profile_id) = profile_id_str.parse::<uuid::Uuid>() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match delete_import_profile::execute(
        &state.app_ctx,
        DeleteImportProfileCommand {
            user_id: user_id.value(),
            profile_id,
        },
    )
    .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            let status = if matches!(e, domain::errors::DomainError::NotFound(_)) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            status.into_response()
        }
    }
}
