use crate::html::helpers::{build_page_context, encode_error};
use crate::html::templates::WatchlistTemplate;
use crate::{
    extractors::{OptionalCookieUser, RequiredCookieUser},
    html::csrf::CsrfToken,
    html::render::render_page,
    state::AppState,
};
use application::{
    diary::commands::MovieInput,
    watchlist::{
        add as add_to_watchlist,
        commands::{AddToWatchlistCommand, RemoveFromWatchlistCommand},
        get_watchlist_for_owner, remove as remove_from_watchlist,
    },
};
use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use domain::errors::DomainError;

pub async fn get_watchlist_page(
    OptionalCookieUser(viewer_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(owner_id): Path<uuid::Uuid>,
    Query(params): Query<crate::html::forms::WatchlistQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, viewer_id.clone(), csrf.0).await;
    let is_owner = viewer_id.map(|u| u.value() == owner_id).unwrap_or(false);

    let view = match get_watchlist_for_owner::execute(
        &state.app_ctx.deps.watchlist.get_watchlist_for_owner,
        owner_id,
        params.limit.or(Some(20)),
        params.offset.or(Some(0)),
    )
    .await
    {
        Ok(view) => view,
        Err(e) => return crate::errors::domain_error_response(e),
    };

    let result = match view {
        get_watchlist_for_owner::WatchlistView::Local(page) => {
            crate::html::mappers::watchlist::build_watchlist_page(page, is_owner)
        }
        get_watchlist_for_owner::WatchlistView::Remote(entries) => {
            crate::html::mappers::watchlist::build_remote_watchlist_page(entries)
        }
    };

    render_page(WatchlistTemplate {
        ctx: &ctx,
        owner_id,
        display_entries: &result.display_entries,
        current_offset: result.current_offset,
        has_more: result.has_more,
        limit: result.limit,
        is_owner,
        error: params.error,
    })
    .into_response()
}

pub async fn post_watchlist_add_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<crate::html::forms::WatchlistAddForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let redirect_base = form
        .redirect_after
        .as_deref()
        .filter(|u| u.starts_with('/') && !u.starts_with("//"))
        .unwrap_or("/")
        .to_string();

    let input = if let Some(id) = form.movie_id {
        MovieInput {
            movie_id: Some(id),
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
        }
    } else {
        let query = form.query.as_deref().unwrap_or("").trim().to_string();
        let is_external_id = query.starts_with("tmdb:")
            || (query.starts_with("tt")
                && query.len() > 2
                && query[2..].chars().all(|c| c.is_ascii_digit()));
        if is_external_id {
            MovieInput {
                movie_id: None,
                external_metadata_id: Some(query),
                manual_title: None,
                manual_release_year: None,
                manual_director: None,
            }
        } else {
            MovieInput {
                movie_id: None,
                external_metadata_id: None,
                manual_title: if query.is_empty() { None } else { Some(query) },
                manual_release_year: form.year,
                manual_director: None,
            }
        }
    };

    match add_to_watchlist::execute(
        &state.app_ctx.deps.watchlist.add,
        AddToWatchlistCommand {
            user_id: user_id.value(),
            input,
        },
    )
    .await
    {
        Ok(()) => Redirect::to(&redirect_base).into_response(),
        Err(DomainError::NotFound(_)) => Redirect::to(&redirect_base).into_response(),
        Err(DomainError::ValidationError(msg)) => {
            let sep = if redirect_base.contains('?') {
                '&'
            } else {
                '?'
            };
            let url = format!("{}{}error={}", redirect_base, sep, encode_error(&msg));
            Redirect::to(&url).into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn post_watchlist_remove_html(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    Path(movie_id): Path<uuid::Uuid>,
    Form(form): Form<crate::html::forms::DeleteRedirectForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match remove_from_watchlist::execute(
        &state.app_ctx.deps.watchlist.remove_from_watchlist,
        RemoveFromWatchlistCommand {
            user_id: user_id.value(),
            movie_id,
        },
    )
    .await
    {
        Ok(()) | Err(DomainError::NotFound(_)) => {
            let redirect_url = form
                .redirect_after
                .filter(|u| u.starts_with('/') && !u.starts_with("//"))
                .unwrap_or_else(|| "/".to_string());
            Redirect::to(&redirect_url).into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}
