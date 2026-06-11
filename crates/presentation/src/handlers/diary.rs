use axum::{
    Form, Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

use application::diary::{
    commands::DeleteReviewCommand,
    delete_review, export_diary as export_diary_uc, get_activity_feed as get_feed_uc, get_diary,
    log_review,
    queries::{ExportQuery, GetActivityFeedQuery},
    deps::{DeleteReviewDeps, GetActivityFeedDeps},
};
use domain::models::ExportFormat;

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, OptionalCookieUser, RequiredCookieUser},
    forms::{ErrorQuery, FeedQueryParams, LogReviewData, LogReviewForm, to_diary_query},
    render::render_page,
    state::AppState,
};
use api_types::{
    ActivityFeedQueryParams, ActivityFeedResponse, DiaryQueryParams, DiaryResponse,
    ExportQueryParams, LogReviewRequest,
};
use template_askama::{ActivityFeedTemplate, NewReviewTemplate, build_page_items};

use super::helpers::build_page_context;

fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

// ── API ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/diary",
    params(DiaryQueryParams),
    responses(
        (status = 200, body = DiaryResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_diary(
    State(state): State<AppState>,
    Query(params): Query<DiaryQueryParams>,
) -> Result<Json<DiaryResponse>, ApiError> {
    let page = get_diary::execute(&state.app_ctx.repos.diary, to_diary_query(params)).await?;

    Ok(Json(DiaryResponse {
        items: page
            .items
            .iter()
            .map(crate::mappers::movies::entry_to_dto)
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

#[utoipa::path(
    post, path = "/api/v1/reviews",
    request_body = LogReviewRequest,
    responses(
        (status = 201, description = "Review created"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_review(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<LogReviewRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = LogReviewData::try_from(req).map_err(ApiError)?;
    log_review::execute(&state.app_ctx.services.review_logger, data.into_command(user.0.value())).await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete, path = "/api/v1/reviews/{id}",
    params(("id" = Uuid, Path, description = "Review ID")),
    responses(
        (status = 204, description = "Review deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Review not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_review(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(review_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    let deps = DeleteReviewDeps {
        review: state.app_ctx.repos.review.clone(),
        diary: state.app_ctx.repos.diary.clone(),
        movie: state.app_ctx.repos.movie.clone(),
        event_publisher: state.app_ctx.services.event_publisher.clone(),
    };
    delete_review::execute(&deps, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/api/v1/diary/export",
    params(ExportQueryParams),
    responses(
        (status = 200, description = "Diary file download", content_type = "text/csv"),
        (status = 400, description = "Invalid format parameter"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn export_diary(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(params): Query<ExportQueryParams>,
) -> impl IntoResponse {
    let format = match params.format.as_str() {
        "csv" => ExportFormat::Csv,
        "json" => ExportFormat::Json,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (content_type, filename) = match &format {
        ExportFormat::Csv => ("text/csv; charset=utf-8", "diary.csv"),
        ExportFormat::Json => ("application/json", "diary.json"),
    };
    let query = ExportQuery {
        user_id: user.0.value(),
        format,
    };
    match export_diary_uc::execute(
        &state.app_ctx.repos.diary,
        &state.app_ctx.services.diary_exporter,
        query,
    )
    .await
    {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, content_type.to_string()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("export error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/activity-feed",
    params(ActivityFeedQueryParams),
    responses((status = 200, body = ActivityFeedResponse)),
)]
pub async fn get_activity_feed(
    State(state): State<AppState>,
    Query(params): Query<ActivityFeedQueryParams>,
) -> Result<Json<ActivityFeedResponse>, ApiError> {
    let deps = GetActivityFeedDeps {
        diary: state.app_ctx.repos.diary.clone(),
        social_query: state.app_ctx.repos.social_query.clone(),
        config: state.app_ctx.config.clone(),
    };
    let page = get_feed_uc::execute(
        &deps,
        GetActivityFeedQuery {
            limit: params.limit.unwrap_or(20),
            offset: params.offset.unwrap_or(0),
            sort_by: params
                .sort_by
                .as_deref()
                .map(|s| s.parse().unwrap_or_default())
                .unwrap_or_default(),
            search: None,
            viewer_user_id: None,
            filter_following: false,
        },
    )
    .await?;
    Ok(Json(ActivityFeedResponse {
        items: page
            .items
            .iter()
            .map(crate::mappers::diary::feed_entry_to_dto)
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_new_review_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id), csrf.0).await;
    ctx.page_title = "Log a Review — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/reviews/new", state.app_ctx.config.base_url);
    render_page(NewReviewTemplate {
        ctx: &ctx,
        error: params.error.as_deref(),
    })
}

pub async fn post_review_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<LogReviewForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let data = match LogReviewData::try_from(form) {
        Ok(d) => d,
        Err(_) => {
            return Redirect::to("/reviews/new?error=Invalid+date+format").into_response();
        }
    };

    match log_review::execute(&state.app_ctx.services.review_logger, data.into_command(user_id.value())).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/reviews/new?error={}", msg)).into_response()
        }
    }
}

pub async fn post_delete_review_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Path(review_id): Path<Uuid>,
    Form(form): Form<crate::forms::DeleteRedirectForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    let deps = DeleteReviewDeps {
        review: state.app_ctx.repos.review.clone(),
        diary: state.app_ctx.repos.diary.clone(),
        movie: state.app_ctx.repos.movie.clone(),
        event_publisher: state.app_ctx.services.event_publisher.clone(),
    };
    match delete_review::execute(&deps, cmd).await {
        Ok(()) => {
            let redirect_url = form
                .redirect_after
                .filter(|url| {
                    (url.starts_with('/') && !url.starts_with("//")) || url.starts_with('?')
                })
                .unwrap_or_else(|| "/".to_string());
            Redirect::to(&redirect_url).into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn get_export_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Query(params): Query<api_types::ExportQueryParams>,
) -> impl IntoResponse {
    let format = match params.format.as_str() {
        "csv" => ExportFormat::Csv,
        "json" => ExportFormat::Json,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (content_type, filename) = match &format {
        ExportFormat::Csv => ("text/csv; charset=utf-8", "diary.csv"),
        ExportFormat::Json => ("application/json", "diary.json"),
    };
    let query = ExportQuery {
        user_id: user_id.value(),
        format,
    };
    match export_diary_uc::execute(
        &state.app_ctx.repos.diary,
        &state.app_ctx.services.diary_exporter,
        query,
    )
    .await
    {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, content_type.to_string()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn get_activity_feed_html(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Query(params): Query<FeedQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let filter_following =
        cfg!(feature = "federation") && params.filter == "following" && user_id.is_some();
    let filter_str = if filter_following { "following" } else { "all" };

    let sort_by_str = match params.sort_by.as_str() {
        "date_asc" => "date_asc",
        "rating" => "rating",
        "rating_asc" => "rating_asc",
        _ => "date",
    };

    let query = application::diary::queries::GetActivityFeedQuery {
        limit,
        offset,
        sort_by: sort_by_str.parse().unwrap_or_default(),
        search: if params.search.is_empty() {
            None
        } else {
            Some(params.search.clone())
        },
        viewer_user_id: user_id.map(|u| u.value()),
        filter_following,
    };

    let deps = GetActivityFeedDeps {
        diary: state.app_ctx.repos.diary.clone(),
        social_query: state.app_ctx.repos.social_query.clone(),
        config: state.app_ctx.config.clone(),
    };

    match application::diary::get_activity_feed::execute(&deps, query).await {
        Ok(entries) => {
            let entry_limit = entries.limit;
            let entry_offset = entries.offset;
            let has_more =
                (entry_offset as u64).saturating_add(entry_limit as u64) < entries.total_count;
            let total_pages = (entries.total_count as u32)
                .saturating_add(entry_limit.saturating_sub(1))
                .checked_div(entry_limit)
                .unwrap_or(1);
            let current_page = entry_offset.checked_div(entry_limit).unwrap_or(0);
            let page_items = build_page_items(total_pages, current_page);
            render_page(ActivityFeedTemplate {
                entries: entries.items.as_slice(),
                current_offset: entry_offset,
                limit: entry_limit,
                has_more,
                ctx: &ctx,
                page_items,
                filter: filter_str.to_string(),
                sort_by: sort_by_str.to_string(),
                search: params.search,
            })
            .into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}
