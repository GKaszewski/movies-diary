use crate::extractors::{Body, Params, PathId};
use crate::support::build_export_response;
use crate::{
    errors::ApiError,
    extractors::AuthenticatedUser,
    html::forms::{LogReviewData, to_diary_query},
    state::AppState,
};
use api_types::{
    ActivityFeedQueryParams, ActivityFeedResponse, DiaryQueryParams, DiaryResponse,
    EditReviewRequest, ExportQueryParams, LogReviewRequest,
};
use application::diary::{
    commands::{DeleteReviewCommand, EditReviewCommand},
    delete_review, edit_review, get_activity_feed as get_feed_uc, get_diary, log_review,
    queries::GetActivityFeedQuery,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

#[utoipa::path(
    get, path = "/api/v1/diary",
    params(DiaryQueryParams),
    responses(
        (status = 200, body = DiaryResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_diary(
    State(state): State<AppState>,
    Params(params): Params<DiaryQueryParams>,
) -> Result<Json<DiaryResponse>, ApiError> {
    let page =
        get_diary::execute(&state.app_ctx.deps.diary.get_diary, to_diary_query(params)).await?;

    Ok(Json(DiaryResponse {
        items: page
            .items
            .iter()
            .map(api_types::mappers::entry_to_dto)
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
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_review(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(req): Body<LogReviewRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = LogReviewData::try_from(req).map_err(ApiError)?;
    log_review::execute(
        &state.app_ctx.services.review_logger,
        data.into_command(user.0.value()),
    )
    .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete, path = "/api/v1/reviews/{id}",
    params(("id" = Uuid, Path, description = "Review ID")),
    responses(
        (status = 204, description = "Review deleted"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden", body = api_types::ErrorResponse),
        (status = 404, description = "Review not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_review(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(review_id): PathId<Uuid>,
) -> Result<StatusCode, ApiError> {
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    delete_review::execute(&state.app_ctx.deps.diary.delete_review, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch, path = "/api/v1/reviews/{id}",
    request_body = EditReviewRequest,
    params(("id" = Uuid, Path, description = "Review ID")),
    responses(
        (status = 200, description = "Review updated"),
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden", body = api_types::ErrorResponse),
        (status = 404, description = "Review not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn patch_review(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(review_id): PathId<Uuid>,
    Body(req): Body<EditReviewRequest>,
) -> Result<StatusCode, ApiError> {
    let watched_at = req
        .watched_at
        .map(|s| domain::value_objects::parse_watched_at(&s).map_err(ApiError))
        .transpose()?;

    let cmd = EditReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
        rating: req.rating,
        comment: req.comment,
        watched_at,
        watch_medium: req.watch_medium,
        spoilers: req.spoilers,
    };
    edit_review::execute(&state.app_ctx.deps.diary.edit_review, cmd).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get, path = "/api/v1/diary/export",
    params(ExportQueryParams),
    responses(
        (status = 200, description = "Diary file download", content_type = "text/csv"),
        (status = 400, description = "Invalid format parameter", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn export_diary(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Params(params): Params<ExportQueryParams>,
) -> impl IntoResponse {
    if !matches!(params.format.as_str(), "csv" | "json") {
        return crate::errors::refuse(
            StatusCode::BAD_REQUEST,
            crate::errors::INVALID_INPUT,
            "format must be csv or json",
        );
    }
    build_export_response(&params.format, user.0.value(), &state)
}

#[utoipa::path(
    get, path = "/api/v1/activity-feed",
    params(ActivityFeedQueryParams),
    responses(
        (status = 200, body = ActivityFeedResponse),
        (status = 400, description = "Invalid query", body = api_types::ErrorResponse),
    ),
)]
pub async fn get_activity_feed(
    State(state): State<AppState>,
    Params(params): Params<ActivityFeedQueryParams>,
) -> Result<Json<ActivityFeedResponse>, ApiError> {
    let page = get_feed_uc::execute(
        &state.app_ctx.deps.diary.get_activity_feed,
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
            .map(api_types::mappers::feed_entry_to_dto)
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}
