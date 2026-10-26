use crate::extractors::{Body, Params, PathId};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::{
    AddToWatchlistRequest, PaginationQueryParams, WatchlistEntryDto, WatchlistResponse,
    WatchlistStatusResponse,
};
use application::{
    diary::commands::MovieInput,
    watchlist::{
        add as add_to_watchlist,
        commands::{AddToWatchlistCommand, RemoveFromWatchlistCommand},
        get as get_watchlist, is_on as is_on_watchlist,
        queries::{GetWatchlistQuery, IsOnWatchlistQuery},
        remove as remove_from_watchlist,
    },
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

#[utoipa::path(
    get, path = "/api/v1/watchlist",
    params(
        ("limit" = Option<u32>, Query, description = "Max results"),
        ("offset" = Option<u32>, Query, description = "Offset"),
    ),
    responses(
        (status = 200, body = WatchlistResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_watchlist_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Params(params): Params<PaginationQueryParams>,
) -> Result<Json<WatchlistResponse>, ApiError> {
    let page = get_watchlist::execute(
        &state.app_ctx.deps.watchlist.get_watchlist,
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
                movie: api_types::mappers::movie_to_dto(&w.movie),
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
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Movie not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_watchlist_add(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(req): Body<AddToWatchlistRequest>,
) -> Result<impl IntoResponse, ApiError> {
    add_to_watchlist::execute(
        &state.app_ctx.deps.watchlist.add,
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
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Not on watchlist", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_watchlist_entry(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    PathId(movie_id): PathId<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    remove_from_watchlist::execute(
        &state.app_ctx.deps.watchlist.remove_from_watchlist,
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
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_watchlist_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    PathId(movie_id): PathId<Uuid>,
) -> Result<Json<WatchlistStatusResponse>, ApiError> {
    let on_watchlist = is_on_watchlist::execute(
        &state.app_ctx.deps.watchlist.is_on_watchlist,
        IsOnWatchlistQuery {
            user_id: user.0.value(),
            movie_id,
        },
    )
    .await?;
    Ok(Json(WatchlistStatusResponse { on_watchlist }))
}
