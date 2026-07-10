use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
};
use uuid::Uuid;

use application::{diary::get_diary, diary::queries::GetDiaryQuery};
use domain::{errors::DomainError, models::ReviewSortBy, value_objects::UserId};

use crate::{errors::ApiError, state::AppState};

pub async fn get_feed(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let query = GetDiaryQuery {
        limit: Some(super::RSS_FEED_LIMIT),
        offset: Some(0),
        sort_by: Some(ReviewSortBy::Descending),
        movie_id: None,
        user_id: None,
    };
    let page = get_diary::execute(&state.app_ctx.repos.diary, query).await?;
    let xml = state
        .rss_renderer
        .render_feed(&page.items, "Movie Diary")
        .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;
    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        xml,
    ))
}

pub async fn get_user_feed(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .app_ctx
        .repos
        .user
        .find_by_id(&UserId::from_uuid(user_id))
        .await
        .map_err(ApiError)?
        .ok_or_else(|| ApiError(DomainError::NotFound(format!("User {user_id}"))))?;

    let query = GetDiaryQuery {
        limit: Some(super::RSS_FEED_LIMIT),
        offset: Some(0),
        sort_by: Some(ReviewSortBy::Descending),
        movie_id: None,
        user_id: Some(user_id),
    };
    let page = get_diary::execute(&state.app_ctx.repos.diary, query).await?;

    let display_name = user.email().value().split('@').next().unwrap_or("User");
    let title = format!("{}'s Movie Diary", display_name);

    let xml = state
        .rss_renderer
        .render_feed(&page.items, &title)
        .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        xml,
    ))
}
