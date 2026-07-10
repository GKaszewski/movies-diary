use axum::{
    Form, Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

use application::{
    diary::commands::MovieInput,
    watchlist::{
        add as add_to_watchlist,
        commands::{AddToWatchlistCommand, RemoveFromWatchlistCommand},
        deps::WatchlistAddDeps,
        get as get_watchlist, is_on as is_on_watchlist,
        queries::{GetWatchlistQuery, IsOnWatchlistQuery},
        remove as remove_from_watchlist,
    },
};
use domain::errors::DomainError;

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, OptionalCookieUser, RequiredCookieUser},
    render::render_page,
    state::AppState,
};
use api_types::{
    AddToWatchlistRequest, PaginationQueryParams, WatchlistEntryDto, WatchlistResponse,
    WatchlistStatusResponse,
};
use template_askama::WatchlistTemplate;

use super::helpers::{build_page_context, encode_error};

// ── API ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/watchlist",
    params(
        ("limit" = Option<u32>, Query, description = "Max results"),
        ("offset" = Option<u32>, Query, description = "Offset"),
    ),
    responses(
        (status = 200, body = WatchlistResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_watchlist_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(params): Query<PaginationQueryParams>,
) -> Result<Json<WatchlistResponse>, ApiError> {
    let page = get_watchlist::execute(
        state.app_ctx.repos.watchlist.clone(),
        GetWatchlistQuery {
            user_id: user.0.value(),
            limit: params.limit,
            offset: params.offset,
        },
    )
    .await?;

    Ok(Json(WatchlistResponse {
        items: page
            .items
            .into_iter()
            .map(|w| WatchlistEntryDto {
                id: w.entry.id.value(),
                movie: crate::mappers::movies::movie_to_dto(&w.movie),
                added_at: w.entry.added_at.to_string(),
            })
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

#[utoipa::path(
    post, path = "/api/v1/watchlist",
    request_body = AddToWatchlistRequest,
    responses(
        (status = 201, description = "Added to watchlist"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Movie not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_watchlist_add(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<AddToWatchlistRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let deps = WatchlistAddDeps {
        movie: state.app_ctx.repos.movie.clone(),
        metadata: state.app_ctx.services.metadata.clone(),
        watchlist: state.app_ctx.repos.watchlist.clone(),
        event_publisher: state.app_ctx.services.event_publisher.clone(),
    };
    add_to_watchlist::execute(
        &deps,
        AddToWatchlistCommand {
            user_id: user.0.value(),
            input: MovieInput {
                movie_id: req.movie_id,
                external_metadata_id: req.external_metadata_id,
                manual_title: req.manual_title,
                manual_release_year: req.manual_release_year,
                manual_director: None,
            },
        },
    )
    .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete, path = "/api/v1/watchlist/{movie_id}",
    params(("movie_id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 204, description = "Removed from watchlist"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not on watchlist"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_watchlist_entry(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(movie_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    remove_from_watchlist::execute(
        state.app_ctx.repos.watchlist.clone(),
        state.app_ctx.services.event_publisher.clone(),
        RemoveFromWatchlistCommand {
            user_id: user.0.value(),
            movie_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/api/v1/watchlist/{movie_id}",
    params(("movie_id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 200, body = WatchlistStatusResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_watchlist_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(movie_id): Path<Uuid>,
) -> Result<Json<WatchlistStatusResponse>, ApiError> {
    let on_watchlist = is_on_watchlist::execute(
        state.app_ctx.repos.watchlist.clone(),
        IsOnWatchlistQuery {
            user_id: user.0.value(),
            movie_id,
        },
    )
    .await?;
    Ok(Json(WatchlistStatusResponse { on_watchlist }))
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_watchlist_page(
    OptionalCookieUser(viewer_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(owner_id): Path<uuid::Uuid>,
    Query(params): Query<crate::forms::WatchlistQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, viewer_id.clone(), csrf.0).await;
    let is_owner = viewer_id.map(|u| u.value() == owner_id).unwrap_or(false);

    let user_id = domain::value_objects::UserId::from_uuid(owner_id);
    let is_local = state
        .app_ctx
        .repos
        .user
        .find_by_id(&user_id)
        .await
        .map(|u| u.is_some())
        .unwrap_or(false);

    let result = if is_local {
        match get_watchlist::execute(
            state.app_ctx.repos.watchlist.clone(),
            application::watchlist::queries::GetWatchlistQuery {
                user_id: owner_id,
                limit: params.limit.or(Some(20)),
                offset: params.offset.or(Some(0)),
            },
        )
        .await
        {
            Ok(page) => crate::mappers::watchlist::build_watchlist_page(page, is_owner),
            Err(e) => return crate::errors::domain_error_response(e),
        }
    } else {
        let remote_entries = state
            .app_ctx
            .repos
            .remote_watchlist
            .get_by_derived_uuid(owner_id)
            .await
            .unwrap_or_default();
        crate::mappers::watchlist::build_remote_watchlist_page(remote_entries)
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
    Form(form): Form<crate::forms::WatchlistAddForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
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

    let deps = WatchlistAddDeps {
        movie: state.app_ctx.repos.movie.clone(),
        metadata: state.app_ctx.services.metadata.clone(),
        watchlist: state.app_ctx.repos.watchlist.clone(),
        event_publisher: state.app_ctx.services.event_publisher.clone(),
    };

    match add_to_watchlist::execute(
        &deps,
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
    Form(form): Form<crate::forms::DeleteRedirectForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match remove_from_watchlist::execute(
        state.app_ctx.repos.watchlist.clone(),
        state.app_ctx.services.event_publisher.clone(),
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
