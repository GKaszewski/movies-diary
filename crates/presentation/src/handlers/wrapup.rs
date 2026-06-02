use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use uuid::Uuid;

use application::wrapup::{
    commands::RequestWrapUpCommand,
    delete as delete_wrapup, generate, get_wrapup,
    list_wrapups::{self, ListWrapUpsQuery},
};
use domain::errors::DomainError;
use domain::models::wrapup::{WrapUpRecord, WrapUpReport, WrapUpStatus};
use domain::value_objects::WrapUpId;

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AdminApiUser, AuthenticatedUser, OptionalCookieUser},
    render::render_page,
    state::AppState,
};
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
        (status = 403, description = "Forbidden — admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_generate(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    Json(req): Json<GenerateWrapUpRequest>,
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
            Some(json) => {
                (StatusCode::OK, [("content-type", "application/json")], json).into_response()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Ok(Some(_)) => StatusCode::ACCEPTED.into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}

#[utoipa::path(
    get, path = "/api/v1/wrapups/{id}/video",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 200, description = "MP4 video file", content_type = "video/mp4"),
        (status = 404, description = "Not found or video not generated"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_video(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let record = match state
        .app_ctx
        .repos
        .wrapup_repo
        .get_by_id(&WrapUpId::from_uuid(id))
        .await
    {
        Ok(Some(r)) if r.status == WrapUpStatus::Ready => r,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let _ = record;
    let video_key = format!("wrapups/{}/video.mp4", id);
    match state
        .app_ctx
        .services
        .image_storage
        .get_stream(&video_key)
        .await
    {
        Ok(stream) => {
            let body = axum::body::Body::from_stream(stream);
            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, "video/mp4"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"wrapup.mp4\"",
                    ),
                ],
                body,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[utoipa::path(
    delete, path = "/api/v1/wrapups/{id}",
    params(("id" = Uuid, Path, description = "Wrap-up ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — admin only"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_wrapup_handler(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    delete_wrapup::execute(&state.app_ctx, WrapUpId::from_uuid(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── HTML handlers ───────────────────────────────────────────────────────────

fn format_watch_time(minutes: u32) -> String {
    let h = minutes / 60;
    let m = minutes % 60;
    if h > 0 && m > 0 {
        format!("{}h {}m", h, m)
    } else if h > 0 {
        format!("{}h", h)
    } else {
        format!("{}m", m)
    }
}

fn render_wrapup(
    report: &WrapUpReport,
    year: i32,
    ctx: &application::ports::HtmlPageContext,
) -> axum::response::Response {
    let rating_max = report
        .rating_distribution
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let rating_pcts: [f64; 5] =
        std::array::from_fn(|i| report.rating_distribution[i] as f64 / rating_max as f64 * 100.0);
    let genre_max = report
        .top_genres
        .first()
        .map(|g| g.count)
        .unwrap_or(1)
        .max(1);
    let genre_pcts: Vec<f64> = report
        .top_genres
        .iter()
        .take(8)
        .map(|g| g.count as f64 / genre_max as f64 * 100.0)
        .collect();
    let tmpl = template_askama::WrapUpPageTemplate {
        ctx,
        report,
        year_label: year.to_string(),
        watch_time_display: format_watch_time(report.total_watch_time_minutes),
        rating_max,
        genre_max,
        rating_pcts,
        genre_pcts,
    };
    render_page(tmpl)
}

pub async fn get_user_wrapup_html(
    OptionalCookieUser(viewer): OptionalCookieUser,
    State(state): State<AppState>,
    Path((user_id, year)): Path<(Uuid, i32)>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let start = match NaiveDate::from_ymd_opt(year, 1, 1) {
        Some(d) => d,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let end = match NaiveDate::from_ymd_opt(year + 1, 1, 1) {
        Some(d) => d,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let record = match state
        .app_ctx
        .repos
        .wrapup_repo
        .find_existing(Some(user_id), start, end)
        .await
    {
        Ok(Some(r)) if r.status == WrapUpStatus::Ready => r,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    let report: WrapUpReport = match &record.report_json {
        Some(json) => match serde_json::from_str(json) {
            Ok(r) => r,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let ctx = super::html::build_page_context(&state, viewer, csrf.0).await;
    render_wrapup(&report, year, &ctx)
}

pub async fn get_global_wrapup_html(
    OptionalCookieUser(viewer): OptionalCookieUser,
    State(state): State<AppState>,
    Path(year): Path<i32>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let start = match NaiveDate::from_ymd_opt(year, 1, 1) {
        Some(d) => d,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let end = match NaiveDate::from_ymd_opt(year + 1, 1, 1) {
        Some(d) => d,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let record = match state
        .app_ctx
        .repos
        .wrapup_repo
        .find_existing(None, start, end)
        .await
    {
        Ok(Some(r)) if r.status == WrapUpStatus::Ready => r,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    let report: WrapUpReport = match &record.report_json {
        Some(json) => match serde_json::from_str(json) {
            Ok(r) => r,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let ctx = super::html::build_page_context(&state, viewer, csrf.0).await;
    render_wrapup(&report, year, &ctx)
}
