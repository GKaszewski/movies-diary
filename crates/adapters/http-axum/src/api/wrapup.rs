use crate::extractors::{Body, PathId};
use crate::{
    errors::ApiError,
    extractors::{AdminApiUser, AuthenticatedUser},
    state::AppState,
};
use api_types::wrapup::{
    GenerateWrapUpRequest, WrapUpGeneratedResponse, WrapUpListResponse, WrapUpStatusResponse,
};
use application::wrapup::{
    commands::RequestWrapUpCommand,
    delete as delete_wrapup, generate, get_wrapup,
    list_wrapups::{self, ListWrapUpsQuery},
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::NaiveDate;
use domain::errors::DomainError;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::value_objects::WrapUpId;
use uuid::Uuid;

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
        (status = 400, description = "Invalid date format", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden — admin only", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_generate(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    Body(req): Body<GenerateWrapUpRequest>,
) -> Result<Json<WrapUpGeneratedResponse>, ApiError> {
    let start = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")
        .map_err(|_| DomainError::ValidationError("invalid start_date".into()))?;
    let end = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d")
        .map_err(|_| DomainError::ValidationError("invalid end_date".into()))?;
    let user_id = req.user_id;
    let cmd = RequestWrapUpCommand {
        user_id,
        start_date: start,
        end_date: end,
    };
    let id = generate::execute(&state.app_ctx.deps.wrapup.generate, cmd).await?;
    Ok(Json(WrapUpGeneratedResponse {
        id: id.value().to_string(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/wrapups",
    responses(
        (status = 200, body = WrapUpListResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_list(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<WrapUpListResponse>, ApiError> {
    let records = list_wrapups::execute(
        &state.app_ctx.deps.wrapup.list_wrapups,
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
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_status(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    PathId(id): PathId<Uuid>,
) -> Result<Json<WrapUpStatusResponse>, ApiError> {
    let record = get_wrapup::execute(
        &state.app_ctx.deps.wrapup.get_wrapup,
        WrapUpId::from_uuid(id),
    )
    .await?
    .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;
    Ok(Json(record_to_dto(&record)))
}

#[utoipa::path(
    get, path = "/api/v1/wrapups/{id}/report",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 200, description = "Report JSON", body = inline(serde_json::Value)),
        (status = 202, description = "Still generating"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_report(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    PathId(id): PathId<Uuid>,
) -> impl IntoResponse {
    match get_wrapup::execute(
        &state.app_ctx.deps.wrapup.get_wrapup,
        WrapUpId::from_uuid(id),
    )
    .await
    {
        Ok(Some(record)) if record.status == WrapUpStatus::Ready => match record.report {
            Some(ref report) => {
                let json = serde_json::to_string(report).unwrap_or_default();
                (StatusCode::OK, [("content-type", "application/json")], json).into_response()
            }
            None => {
                crate::errors::refuse(StatusCode::NOT_FOUND, crate::errors::NOT_FOUND, "not found")
            }
        },
        Ok(Some(_)) => StatusCode::ACCEPTED.into_response(),
        Ok(None) => {
            crate::errors::refuse(StatusCode::NOT_FOUND, crate::errors::NOT_FOUND, "not found")
        }
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

#[utoipa::path(
    delete, path = "/api/v1/wrapups/{id}",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden — admin only", body = api_types::ErrorResponse),
        (status = 404, description = "Not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_wrapup_handler(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    PathId(id): PathId<Uuid>,
) -> Result<StatusCode, ApiError> {
    delete_wrapup::execute(
        &state.app_ctx.deps.wrapup.delete_wrapup,
        WrapUpId::from_uuid(id),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
