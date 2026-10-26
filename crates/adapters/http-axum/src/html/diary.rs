use crate::html::helpers::{build_page_context, encode_domain_error};
use crate::html::templates::{ActivityFeedTemplate, NewReviewTemplate, build_page_items};
use crate::support::build_export_response;
use crate::{
    extractors::{OptionalCookieUser, RequiredCookieUser},
    html::csrf::CsrfToken,
    html::forms::{ErrorQuery, FeedQueryParams, LogReviewData, LogReviewForm},
    html::render::render_page,
    state::AppState,
};
use application::diary::{commands::DeleteReviewCommand, delete_review, log_review};
use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let data = match LogReviewData::try_from(form) {
        Ok(d) => d,
        Err(_) => {
            return Redirect::to("/reviews/new?error=Invalid+date+format").into_response();
        }
    };

    match log_review::execute(
        &state.app_ctx.services.review_logger,
        data.into_command(user_id.value()),
    )
    .await
    {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            let msg = encode_domain_error(&e);
            Redirect::to(&format!("/reviews/new?error={}", msg)).into_response()
        }
    }
}

pub async fn post_delete_review_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Path(review_id): Path<Uuid>,
    Form(form): Form<crate::html::forms::DeleteRedirectForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    match delete_review::execute(&state.app_ctx.deps.diary.delete_review, cmd).await {
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
    build_export_response(&params.format, user_id.value(), &state)
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

    let filter_following = params.filter == "following" && user_id.is_some();
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

    match application::diary::get_activity_feed::execute(
        &state.app_ctx.deps.diary.get_activity_feed,
        query,
    )
    .await
    {
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
