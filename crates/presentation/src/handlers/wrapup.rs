use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use uuid::Uuid;

use application::wrapup::{
    commands::RequestWrapUpCommand,
    generate, get_wrapup,
    list_wrapups::{self, ListWrapUpsQuery},
};
use domain::errors::DomainError;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::value_objects::WrapUpId;

use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::wrapup::{
    GenerateWrapUpRequest, WrapUpGeneratedResponse, WrapUpListResponse, WrapUpStatusResponse,
};

fn record_to_dto(r: &WrapUpRecord) -> WrapUpStatusResponse {
    WrapUpStatusResponse {
        id: r.id.value().to_string(),
        user_id: r.user_id.map(|u| u.to_string()),
        status: format!("{:?}", r.status),
        start_date: r.start_date.to_string(),
        end_date: r.end_date.to_string(),
        created_at: r.created_at.to_string(),
        completed_at: r.completed_at.map(|t| t.to_string()),
        error_message: r.error_message.clone(),
    }
}

#[utoipa::path(
    post, path = "/api/v1/wrapups/generate",
    request_body = GenerateWrapUpRequest,
    responses(
        (status = 200, body = WrapUpGeneratedResponse),
        (status = 400, description = "Invalid date format"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_generate(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<GenerateWrapUpRequest>,
) -> Result<Json<WrapUpGeneratedResponse>, ApiError> {
    let start = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")
        .map_err(|_| DomainError::ValidationError("invalid start_date".into()))?;
    let end = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d")
        .map_err(|_| DomainError::ValidationError("invalid end_date".into()))?;
    let user_id = if req.global.unwrap_or(false) {
        None
    } else {
        Some(user.0.value())
    };
    let cmd = RequestWrapUpCommand {
        user_id,
        start_date: start,
        end_date: end,
    };
    let id = generate::execute(&state.app_ctx, cmd).await?;
    Ok(Json(WrapUpGeneratedResponse {
        id: id.value().to_string(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/wrapups",
    responses(
        (status = 200, body = WrapUpListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<WrapUpListResponse>, ApiError> {
    let records = list_wrapups::execute(
        &state.app_ctx,
        ListWrapUpsQuery {
            user_id: Some(user.0.value()),
        },
    )
    .await?;
    Ok(Json(WrapUpListResponse {
        items: records.iter().map(record_to_dto).collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/wrapups/{id}",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 200, body = WrapUpStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_status(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WrapUpStatusResponse>, ApiError> {
    let record = get_wrapup::execute(&state.app_ctx, WrapUpId::from_uuid(id))
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;
    Ok(Json(record_to_dto(&record)))
}

#[utoipa::path(
    get, path = "/api/v1/wrapups/{id}/report",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 200, description = "Report JSON", content_type = "application/json"),
        (status = 202, description = "Still generating"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_report(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_wrapup::execute(&state.app_ctx, WrapUpId::from_uuid(id)).await {
        Ok(Some(record)) if record.status == WrapUpStatus::Ready => match record.report_json {
            Some(json) => (
                StatusCode::OK,
                [("content-type", "application/json")],
                json,
            )
                .into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Ok(Some(_)) => StatusCode::ACCEPTED.into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}
