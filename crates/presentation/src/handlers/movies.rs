use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use application::{
    diary::{
        commands::SyncPosterCommand,
        get_movie_social_page, get_review_history,
        queries::{GetMovieSocialPageQuery, GetReviewHistoryQuery},
    },
    movies::{get_movies, queries::GetMoviesQuery, sync_poster},
    watchlist::{is_on as is_on_watchlist, queries::IsOnWatchlistQuery},
};
use domain::services::review_history::Trend;

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, OptionalCookieUser},
    render::render_page,
    state::AppState,
};
use api_types::{
    CastMemberDto, CrewMemberDto, GenreDto, KeywordDto, MovieDetailResponse, MovieProfileResponse,
    MovieStatsDto, MoviesQueryParams, MoviesResponse, PaginationQueryParams, ReviewHistoryResponse,
    SocialFeedResponse, SocialReviewDto,
};
use template_askama::MovieDetailTemplate;

use super::helpers::build_page_context;

// ── API ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/movies",
    params(MoviesQueryParams),
    responses(
        (status = 200, body = MoviesResponse),
    )
)]
pub async fn list_movies(
    State(state): State<AppState>,
    Query(params): Query<MoviesQueryParams>,
) -> Result<Json<MoviesResponse>, ApiError> {
    let page = get_movies::execute(
        &state.app_ctx,
        GetMoviesQuery {
            limit: params.limit,
            offset: params.offset,
            search: params.search,
            genre: params.genre,
            language: params.language,
        },
    )
    .await?;

    Ok(Json(MoviesResponse {
        items: page
            .items
            .iter()
            .map(crate::mappers::movies::summary_to_dto)
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

#[utoipa::path(
    get, path = "/api/v1/movies/{id}/history",
    params(("id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 200, body = ReviewHistoryResponse),
        (status = 404, description = "Movie not found"),
    )
)]
pub async fn get_review_history(
    State(state): State<AppState>,
    Path(movie_id): Path<Uuid>,
) -> Result<Json<ReviewHistoryResponse>, ApiError> {
    let (history, trend) =
        get_review_history::execute(&state.app_ctx, GetReviewHistoryQuery { movie_id }).await?;

    Ok(Json(ReviewHistoryResponse {
        movie: crate::mappers::movies::movie_to_dto(history.movie()),
        viewings: history
            .viewings()
            .iter()
            .map(crate::mappers::movies::review_to_dto)
            .collect(),
        trend: match trend {
            Trend::Improved => "improved",
            Trend::Declined => "declined",
            Trend::Neutral => "neutral",
        }
        .to_string(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/movies/{id}/sync-poster",
    params(("id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 204, description = "Poster synced"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Movie not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn sync_poster(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(movie_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    sync_poster::execute(&state.app_ctx, SyncPosterCommand { movie_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/api/v1/movies/{movie_id}",
    params(("movie_id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 200, body = MovieDetailResponse),
        (status = 404, description = "Movie not found"),
    )
)]
pub async fn get_movie_detail(
    State(state): State<AppState>,
    Path(movie_id): Path<Uuid>,
    Query(params): Query<PaginationQueryParams>,
) -> Result<Json<MovieDetailResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let result = get_movie_social_page::execute(
        &state.app_ctx,
        GetMovieSocialPageQuery {
            movie_id,
            limit,
            offset,
        },
    )
    .await?;

    Ok(Json(MovieDetailResponse {
        movie: crate::mappers::movies::movie_to_dto(&result.movie),
        stats: MovieStatsDto {
            total_count: result.stats.total_count,
            avg_rating: result.stats.avg_rating,
            federated_count: result.stats.federated_count,
            rating_histogram: result.stats.rating_histogram,
        },
        reviews: SocialFeedResponse {
            items: result
                .reviews
                .items
                .iter()
                .map(|e| SocialReviewDto {
                    user_display: e.user_display_name().to_string(),
                    rating: e.review().rating().value(),
                    comment: e.review().comment().map(|c| c.value().to_string()),
                    watched_at: e.review().watched_at().to_string(),
                    is_federated: e.review().is_remote(),
                })
                .collect(),
            total_count: result.reviews.total_count,
            limit: result.reviews.limit,
            offset: result.reviews.offset,
        },
    }))
}

#[utoipa::path(
    get, path = "/api/v1/movies/{id}/profile",
    params(("id" = Uuid, Path, description = "Movie ID")),
    responses(
        (status = 200, body = MovieProfileResponse),
        (status = 404, description = "No profile found for this movie"),
    )
)]
pub async fn get_movie_profile(
    State(state): State<AppState>,
    Path(movie_id): Path<Uuid>,
) -> impl IntoResponse {
    use application::movies::get_movie_profile;
    let query = get_movie_profile::GetMovieProfileQuery { movie_id };
    match get_movie_profile::execute(&state.app_ctx, query).await {
        Ok(Some(result)) => {
            let p = result.profile;
            Json(MovieProfileResponse {
                tmdb_id: p.tmdb_id,
                imdb_id: p.imdb_id,
                overview: p.overview,
                tagline: p.tagline,
                runtime_minutes: p.runtime_minutes,
                budget_usd: p.budget_usd,
                revenue_usd: p.revenue_usd,
                vote_average: p.vote_average,
                vote_count: p.vote_count,
                original_language: p.original_language,
                collection_name: p.collection_name,
                genres: p
                    .genres
                    .into_iter()
                    .map(|g| GenreDto {
                        tmdb_id: g.tmdb_id,
                        name: g.name,
                    })
                    .collect(),
                keywords: p
                    .keywords
                    .into_iter()
                    .map(|k| KeywordDto {
                        tmdb_id: k.tmdb_id,
                        name: k.name,
                    })
                    .collect(),
                cast: result
                    .cast
                    .into_iter()
                    .map(|c| CastMemberDto {
                        person_id: c.person_id.value().to_string(),
                        tmdb_person_id: c.tmdb_person_id,
                        name: c.name,
                        character: c.character,
                        billing_order: c.billing_order,
                        profile_path: c.profile_path,
                    })
                    .collect(),
                crew: result
                    .crew
                    .into_iter()
                    .map(|c| CrewMemberDto {
                        person_id: c.person_id.value().to_string(),
                        tmdb_person_id: c.tmdb_person_id,
                        name: c.name,
                        job: c.job,
                        department: c.department,
                        profile_path: c.profile_path,
                    })
                    .collect(),
                enriched_at: p.enriched_at.to_rfc3339(),
            })
            .into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_movie_detail_html(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(movie_id): Path<uuid::Uuid>,
    Query(params): Query<api_types::PaginationQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    match get_movie_social_page::execute(
        &state.app_ctx,
        GetMovieSocialPageQuery {
            movie_id,
            limit,
            offset,
        },
    )
    .await
    {
        Err(e) => crate::errors::domain_error_response(e),
        Ok(result) => {
            let histogram_max = result
                .stats
                .rating_histogram
                .iter()
                .copied()
                .max()
                .unwrap_or(1);
            let has_more =
                result.reviews.offset + result.reviews.limit < result.reviews.total_count as u32;
            let on_watchlist = match &user_id {
                Some(uid) => is_on_watchlist::execute(
                    &state.app_ctx,
                    IsOnWatchlistQuery {
                        user_id: uid.value(),
                        movie_id,
                    },
                )
                .await
                .unwrap_or(false),
                None => false,
            };
            let current_offset = result.reviews.offset;
            let reviews_limit = result.reviews.limit;
            render_page(MovieDetailTemplate {
                ctx: &ctx,
                movie: &result.movie,
                stats: &result.stats,
                profile: result.profile.as_ref(),
                reviews: result.reviews.items.as_slice(),
                on_watchlist,
                current_offset,
                has_more,
                limit: reviews_limit,
                histogram_max,
            })
            .into_response()
        }
    }
}
