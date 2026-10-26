use crate::{errors::ApiError, state::AppState};
use application::{diary::get_diary, diary::get_user_feed, diary::queries::GetDiaryQuery};
use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
};
use domain::{models::ReviewSortBy, ports::FeedChannel};
use uuid::Uuid;

const RSS_FEED_LIMIT: u32 = 50;

pub async fn get_feed(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let query = GetDiaryQuery {
        limit: Some(RSS_FEED_LIMIT),
        offset: Some(0),
        sort_by: Some(ReviewSortBy::Descending),
        movie_id: None,
        user_id: None,
    };
    let page = get_diary::execute(&state.app_ctx.deps.diary.get_diary, query).await?;
    let xml = state
        .rss_renderer
        .render_feed(
            &page.items,
            &FeedChannel {
                title: "Movie Diary",
                link_path: "/",
                self_path: "/feed.rss",
            },
        )
        .map_err(ApiError)?;
    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    ))
}

pub async fn get_user_feed(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let feed = get_user_feed::execute(
        &state.app_ctx.deps.diary.get_user_feed,
        user_id,
        RSS_FEED_LIMIT,
    )
    .await?;

    let title = format!("{}'s Movie Diary", feed.author.display_name);
    let link_path = format!("/users/{user_id}");
    let self_path = format!("/users/{user_id}/feed.rss");

    let xml = state
        .rss_renderer
        .render_feed(
            &feed.entries,
            &FeedChannel {
                title: &title,
                link_path: &link_path,
                self_path: &self_path,
            },
        )
        .map_err(ApiError)?;

    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    ))
}
