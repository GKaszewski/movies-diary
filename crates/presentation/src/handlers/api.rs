use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use std::str::FromStr;

use application::{
    commands::{
        AddToWatchlistCommand, DeleteReviewCommand, MovieInput, RegisterCommand,
        RemoveFromWatchlistCommand, SyncPosterCommand,
    },
    queries::{
        ExportQuery, GetActivityFeedQuery, GetMovieSocialPageQuery, GetMoviesQuery,
        GetReviewHistoryQuery, GetUserProfileQuery, GetUsersQuery, GetWatchlistQuery,
        IsOnWatchlistQuery, LoginQuery,
    },
    use_cases::{
        add_to_watchlist, delete_review, export_diary as export_diary_uc,
        get_activity_feed as get_feed_uc, get_diary, get_movie_social_page, get_movies, get_person,
        get_person_credits, get_review_history, get_user_profile as get_user_profile_uc, get_users,
        get_watchlist, is_on_watchlist, log_review, login as login_uc, register as register_uc,
        remove_from_watchlist, search as search_uc, sync_poster, update_profile,
        update_profile_fields,
    },
};
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, ExportFormat, Movie, MovieSummary, PersonId, Review, collections::PageParams,
    },
    services::review_history::Trend,
    value_objects::UserId,
};

use crate::{
    errors::ApiError,
    extractors::AuthenticatedUser,
    forms::{LogReviewData, to_diary_query},
    state::AppState,
};
use api_types::search::{
    CastCreditDto, CrewCreditDto, MovieSearchHitDto, PaginatedMovieHits, PaginatedPersonHits,
    PersonCreditsDto, PersonDto, PersonSearchHitDto, SearchQueryParams, SearchResponse,
};
use api_types::{
    ActivityFeedQueryParams, ActivityFeedResponse, AddToWatchlistRequest, CastMemberDto,
    CrewMemberDto, DiaryEntryDto, DiaryQueryParams, DiaryResponse, DirectorStatDto,
    ExportQueryParams, FeedEntryDto, GenreDto, KeywordDto, LogReviewRequest, LoginRequest,
    LoginResponse, MonthActivityDto, MonthlyRatingDto, MovieDetailResponse, MovieDto,
    MovieProfileResponse, MovieStatsDto, MoviesQueryParams, MoviesResponse, PaginationQueryParams,
    ProfileResponse, RegisterRequest, ReviewDto, ReviewHistoryResponse, SocialFeedResponse,
    SocialReviewDto, UserProfileQueryParams, UserProfileResponse, UserStatsDto, UserSummaryDto,
    UserTrendsDto, UsersResponse, WatchlistEntryDto, WatchlistResponse, WatchlistStatusResponse,
};
#[cfg(feature = "federation")]
use api_types::{
    ActorListResponse, ActorUrlRequest, AddBlockedDomainRequest, BlockedActorResponse,
    BlockedDomainResponse, FollowRequest, RemoteActorDto,
};

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
    let page = get_diary::execute(&state.app_ctx, to_diary_query(params)).await?;

    Ok(Json(DiaryResponse {
        items: page.items.iter().map(entry_to_dto).collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

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
        items: page.items.iter().map(summary_to_dto).collect(),
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
        movie: movie_to_dto(history.movie()),
        viewings: history.viewings().iter().map(review_to_dto).collect(),
        trend: match trend {
            Trend::Improved => "improved",
            Trend::Declined => "declined",
            Trend::Neutral => "neutral",
        }
        .to_string(),
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
    log_review::execute(&state.app_ctx, data.into_command(user.0.value())).await?;
    Ok(StatusCode::CREATED)
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
    post, path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let result = login_uc::execute(
        &state.app_ctx,
        LoginQuery {
            email: req.email,
            password: req.password,
        },
    )
    .await?;
    Ok(Json(LoginResponse {
        token: result.token,
        user_id: result.user_id,
        email: result.email,
        expires_at: result.expires_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<StatusCode, ApiError> {
    register_uc::execute(
        &state.app_ctx,
        RegisterCommand {
            email: req.email,
            username: req.username,
            password: req.password,
            role: domain::models::UserRole::Standard,
        },
    )
    .await?;
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
) -> impl IntoResponse {
    let cmd = DeleteReviewCommand {
        review_id,
        requesting_user_id: user_id.value(),
    };
    match delete_review::execute(&state.app_ctx, cmd).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => {
            tracing::error!("delete_review error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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
        movie: movie_to_dto(&result.movie),
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
    let id = domain::value_objects::MovieId::from_uuid(movie_id);
    match state
        .app_ctx
        .movie_profile_repository
        .get_by_movie_id(&id)
        .await
    {
        Ok(Some(p)) => Json(MovieProfileResponse {
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
            cast: p
                .cast
                .into_iter()
                .map(|c| CastMemberDto {
                    tmdb_person_id: c.tmdb_person_id,
                    name: c.name,
                    character: c.character,
                    billing_order: c.billing_order,
                    profile_path: c.profile_path,
                })
                .collect(),
            crew: p
                .crew
                .into_iter()
                .map(|c| CrewMemberDto {
                    tmdb_person_id: c.tmdb_person_id,
                    name: c.name,
                    job: c.job,
                    department: c.department,
                    profile_path: c.profile_path,
                })
                .collect(),
            enriched_at: p.enriched_at.to_rfc3339(),
        })
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_movie_profile: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/profile",
    responses(
        (status = 200, body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> impl IntoResponse {
    match application::use_cases::get_current_profile::execute(
        &state.app_ctx,
        application::queries::GetCurrentProfileQuery {
            user_id: user_id.value(),
        },
    )
    .await
    {
        Ok(profile) => Json(ProfileResponse {
            username: profile.username,
            bio: profile.bio,
            avatar_url: profile.avatar_url,
        })
        .into_response(),
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_profile error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    put, path = "/api/v1/profile",
    responses(
        (status = 204, description = "Profile updated"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut display_name: Option<String> = None;
    let mut bio: Option<String> = None;
    let mut avatar_bytes: Option<Vec<u8>> = None;
    let mut avatar_content_type: Option<String> = None;
    let mut banner_bytes: Option<Vec<u8>> = None;
    let mut banner_content_type: Option<String> = None;
    let mut also_known_as: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "display_name" => {
                if let Ok(text) = field.text().await {
                    display_name = Some(text).filter(|s| !s.is_empty());
                }
            }
            "bio" => {
                if let Ok(text) = field.text().await {
                    bio = Some(text);
                }
            }
            "also_known_as" => {
                if let Ok(text) = field.text().await {
                    also_known_as = Some(text).filter(|s| !s.is_empty());
                }
            }
            "avatar" => {
                let ct = field.content_type().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await
                    && !bytes.is_empty()
                {
                    avatar_bytes = Some(bytes.to_vec());
                    avatar_content_type = ct;
                }
            }
            "banner" => {
                let ct = field.content_type().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await
                    && !bytes.is_empty()
                {
                    banner_bytes = Some(bytes.to_vec());
                    banner_content_type = ct;
                }
            }
            _ => {}
        }
    }

    let cmd = application::commands::UpdateProfileCommand {
        user_id: user_id.value(),
        display_name,
        bio,
        avatar_bytes,
        avatar_content_type,
        banner_bytes,
        banner_content_type,
        also_known_as,
    };

    match update_profile::execute(&state.app_ctx, cmd).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(domain::errors::DomainError::ValidationError(msg)) => {
            tracing::warn!("update_profile validation: {}", msg);
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(e) => {
            tracing::error!("update_profile error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    put, path = "/api/v1/profile/fields",
    request_body = api_types::UpdateProfileFieldsRequest,
    responses(
        (status = 204, description = "Profile fields updated"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_fields_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let raw_fields = match body.get("fields").and_then(|f| f.as_array()) {
        Some(arr) => arr.clone(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let fields: Vec<domain::models::ProfileField> = raw_fields
        .iter()
        .filter_map(|f| {
            let name = f.get("name").and_then(|n| n.as_str())?.to_string();
            let value = f.get("value").and_then(|v| v.as_str())?.to_string();
            Some(domain::models::ProfileField { name, value })
        })
        .collect();

    let cmd = application::commands::UpdateProfileFieldsCommand {
        user_id: user_id.value(),
        fields,
    };

    match update_profile_fields::execute(&state.app_ctx, cmd).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(domain::errors::DomainError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, msg).into_response()
        }
        Err(e) => {
            tracing::error!("update_profile_fields error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn movie_to_dto(movie: &Movie) -> MovieDto {
    MovieDto {
        id: movie.id().value(),
        title: movie.title().value().to_string(),
        release_year: movie.release_year().value(),
        director: movie.director().map(|d| d.to_string()),
        poster_path: movie.poster_path().map(|p| p.value().to_string()),
        genres: vec![],
        runtime_minutes: None,
        original_language: None,
        overview: None,
        collection_name: None,
    }
}

fn summary_to_dto(summary: &MovieSummary) -> MovieDto {
    MovieDto {
        id: summary.movie.id().value(),
        title: summary.movie.title().value().to_string(),
        release_year: summary.movie.release_year().value(),
        director: summary.movie.director().map(|d| d.to_string()),
        poster_path: summary.movie.poster_path().map(|p| p.value().to_string()),
        genres: summary.genres.clone(),
        runtime_minutes: summary.runtime_minutes,
        original_language: summary.original_language.clone(),
        overview: summary.overview.clone(),
        collection_name: summary.collection_name.clone(),
    }
}

fn review_to_dto(review: &Review) -> ReviewDto {
    ReviewDto {
        id: review.id().value(),
        rating: review.rating().value(),
        comment: review.comment().map(|c| c.value().to_string()),
        watched_at: review.watched_at().to_string(),
    }
}

fn entry_to_dto(entry: &DiaryEntry) -> DiaryEntryDto {
    DiaryEntryDto {
        movie: movie_to_dto(entry.movie()),
        review: review_to_dto(entry.review()),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/admin/blocked-domains",
    responses(
        (status = 200, body = Vec<BlockedDomainResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_blocked_domains_admin(
    State(state): State<AppState>,
    _admin: crate::extractors::AdminUser,
) -> impl IntoResponse {
    match state.ap_service.get_blocked_domains().await {
        Ok(domains) => {
            let response: Vec<BlockedDomainResponse> = domains
                .into_iter()
                .map(|d| BlockedDomainResponse {
                    domain: d.domain,
                    reason: d.reason,
                    blocked_at: d.blocked_at,
                })
                .collect();
            axum::Json(response).into_response()
        }
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/admin/blocked-domains",
    request_body = AddBlockedDomainRequest,
    responses(
        (status = 201, description = "Domain blocked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: crate::extractors::AdminUser,
    axum::Json(body): axum::Json<AddBlockedDomainRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .add_blocked_domain(&body.domain, body.reason.as_deref())
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    delete, path = "/api/v1/admin/blocked-domains/{domain}",
    params(("domain" = String, Path, description = "Domain to unblock")),
    responses(
        (status = 204, description = "Domain unblocked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: crate::extractors::AdminUser,
    axum::extract::Path(domain): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.ap_service.remove_blocked_domain(&domain).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/block",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor blocked"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn block_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::Json(body): axum::Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .block_actor(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/unblock",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor unblocked"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unblock_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::Json(body): axum::Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .unblock_actor(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/social/blocked",
    responses(
        (status = 200, body = Vec<BlockedActorResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_blocked_actors_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    match state.ap_service.get_blocked_actors(user.0.value()).await {
        Ok(actors) => {
            let response: Vec<BlockedActorResponse> = actors
                .into_iter()
                .map(|a| BlockedActorResponse {
                    url: a.url,
                    handle: a.handle,
                    display_name: a.display_name,
                    avatar_url: a.avatar_url,
                })
                .collect();
            axum::Json(response).into_response()
        }
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
fn ap_err(e: anyhow::Error) -> impl IntoResponse {
    tracing::error!("ActivityPub error: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/social/following",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_following(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    match state.ap_service.get_following(user.0.value()).await {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/social/followers",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_followers(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    match state
        .ap_service
        .get_accepted_followers(user.0.value())
        .await
    {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/follow",
    request_body = FollowRequest,
    responses(
        (status = 200, description = "Follow request sent"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn follow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<FollowRequest>,
) -> impl IntoResponse {
    match state.ap_service.follow(user.0.value(), &body.handle).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/unfollow",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Unfollowed"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unfollow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .unfollow(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/followers/accept",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower accepted"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn accept_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .accept_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/followers/reject",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower rejected"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn reject_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .reject_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/followers/remove",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower removed"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .remove_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/social/followers/pending",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_followers(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    match state.ap_service.get_pending_followers(user.0.value()).await {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
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
    let page = get_feed_uc::execute(
        &state.app_ctx,
        GetActivityFeedQuery {
            limit: params.limit.unwrap_or(20),
            offset: params.offset.unwrap_or(0),
            sort_by: domain::ports::FeedSortBy::Date,
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
            .map(|e| FeedEntryDto {
                movie: movie_to_dto(e.movie()),
                review: review_to_dto(e.review()),
                user_email: e.user_email().to_string(),
                user_display_name: e.user_display_name().to_string(),
            })
            .collect(),
        total_count: page.total_count,
        limit: page.limit,
        offset: page.offset,
    }))
}

#[utoipa::path(
    get, path = "/api/v1/users",
    responses((status = 200, body = UsersResponse)),
)]
pub async fn list_users(State(state): State<AppState>) -> Result<Json<UsersResponse>, ApiError> {
    let result = get_users::execute(&state.app_ctx, GetUsersQuery).await?;
    Ok(Json(UsersResponse {
        users: result
            .users
            .iter()
            .map(|u| UserSummaryDto {
                id: u.user_id.value(),
                email: u.email().to_string(),
                total_movies: u.total_movies,
                avg_rating: u.avg_rating,
            })
            .collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID"),
        UserProfileQueryParams,
    ),
    responses(
        (status = 200, body = UserProfileResponse),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(params): Query<UserProfileQueryParams>,
) -> impl IntoResponse {
    let view_str = params.view.as_deref().unwrap_or("recent");
    let profile_view = match application::queries::ProfileView::from_str(view_str) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let user = match state
        .app_ctx
        .user_repository
        .find_by_id(&UserId::from_uuid(user_id))
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("user lookup: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let profile = match get_user_profile_uc::execute(
        &state.app_ctx,
        GetUserProfileQuery {
            user_id,
            view: profile_view,
            limit: params.limit,
            offset: params.offset,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            is_own_profile: false,
        },
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("profile: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let entries = profile.entries.map(|p| DiaryResponse {
        items: p.items.iter().map(entry_to_dto).collect(),
        total_count: p.total_count,
        limit: p.limit,
        offset: p.offset,
    });

    let history = profile.history.map(|months| {
        months
            .into_iter()
            .map(|m| MonthActivityDto {
                year_month: m.year_month,
                month_label: m.month_label,
                count: m.count,
                entries: m.entries.iter().map(entry_to_dto).collect(),
            })
            .collect()
    });

    let trends = profile.trends.map(|t| UserTrendsDto {
        monthly_ratings: t
            .monthly_ratings
            .into_iter()
            .map(|r| MonthlyRatingDto {
                year_month: r.year_month,
                month_label: r.month_label,
                avg_rating: r.avg_rating,
                count: r.count,
            })
            .collect(),
        top_directors: t
            .top_directors
            .into_iter()
            .map(|d| DirectorStatDto {
                director: d.director,
                count: d.count,
            })
            .collect(),
        max_director_count: t.max_director_count,
    });

    Json(UserProfileResponse {
        user_id,
        username: user.username().value().to_string(),
        stats: UserStatsDto {
            total_movies: profile.stats.total_movies,
            avg_rating: profile.stats.avg_rating,
            favorite_director: profile.stats.favorite_director,
            most_active_month: profile.stats.most_active_month,
        },
        following_count: profile.following_count,
        followers_count: profile.followers_count,
        entries,
        history,
        trends,
    })
    .into_response()
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
    match export_diary_uc::execute(&state.app_ctx, query).await {
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

// Search and person endpoints are intentionally public — browsing the catalog
// and people profiles does not require authentication.

#[utoipa::path(
    get, path = "/api/v1/search",
    params(api_types::search::SearchQueryParams),
    responses(
        (status = 200, body = api_types::search::SearchResponse),
    ),
    tag = "search",
)]
pub async fn get_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let query = domain::models::SearchQuery {
        text: params.q,
        filters: domain::models::SearchFilters {
            genre: params.genre,
            year: params.year,
            person_id: params.person_id.map(PersonId::from_uuid),
            department: params.department,
            language: params.language,
        },
        page: PageParams {
            limit: params.limit.unwrap_or(5),
            offset: params.offset.unwrap_or(0),
        },
    };

    match search_uc::execute(&state.app_ctx, query).await {
        Ok(results) => axum::Json(SearchResponse {
            movies: PaginatedMovieHits {
                items: results
                    .movies
                    .items
                    .iter()
                    .map(|h| MovieSearchHitDto {
                        movie_id: h.movie_id.value(),
                        title: h.title.clone(),
                        release_year: h.release_year,
                        director: h.director.clone(),
                        poster_path: h.poster_path.clone(),
                        genres: h.genres.clone(),
                    })
                    .collect(),
                total_count: results.movies.total_count,
                limit: results.movies.limit,
                offset: results.movies.offset,
            },
            people: PaginatedPersonHits {
                items: results
                    .people
                    .items
                    .iter()
                    .map(|h| PersonSearchHitDto {
                        person_id: h.person_id.value(),
                        name: h.name.clone(),
                        known_for_department: h.known_for_department.clone(),
                        profile_path: h.profile_path.clone(),
                        known_for_titles: h.known_for_titles.clone(),
                    })
                    .collect(),
                total_count: results.people.total_count,
                limit: results.people.limit,
                offset: results.people.offset,
            },
        })
        .into_response(),
        Err(e) => {
            tracing::error!("search failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/people/{id}",
    params(("id" = Uuid, Path, description = "Person ID")),
    responses(
        (status = 200, body = api_types::search::PersonDto),
        (status = 404, description = "Person not found"),
    ),
    tag = "search",
)]
pub async fn get_person_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match get_person::execute(&state.app_ctx, PersonId::from_uuid(id)).await {
        Ok(Some(person)) => axum::Json(PersonDto {
            id: person.id().value(),
            external_id: person.external_id().value().to_string(),
            name: person.name().to_string(),
            known_for_department: person.known_for_department().map(str::to_string),
            profile_path: person.profile_path().map(str::to_string),
        })
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_person failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/people/{id}/credits",
    params(("id" = Uuid, Path, description = "Person ID")),
    responses(
        (status = 200, body = api_types::search::PersonCreditsDto),
        (status = 404, description = "Person not found"),
    ),
    tag = "search",
)]
pub async fn get_person_credits_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match get_person_credits::execute(&state.app_ctx, PersonId::from_uuid(id)).await {
        Ok(credits) => axum::Json(PersonCreditsDto {
            person: PersonDto {
                id: credits.person.id().value(),
                external_id: credits.person.external_id().value().to_string(),
                name: credits.person.name().to_string(),
                known_for_department: credits.person.known_for_department().map(str::to_string),
                profile_path: credits.person.profile_path().map(str::to_string),
            },
            cast: credits
                .cast
                .iter()
                .map(|c| CastCreditDto {
                    movie_id: c.movie_id.value(),
                    title: c.title.clone(),
                    release_year: c.release_year,
                    character: c.character.clone(),
                    poster_path: c.poster_path.clone(),
                })
                .collect(),
            crew: credits
                .crew
                .iter()
                .map(|c| CrewCreditDto {
                    movie_id: c.movie_id.value(),
                    title: c.title.clone(),
                    release_year: c.release_year,
                    job: c.job.clone(),
                    department: c.department.clone(),
                    poster_path: c.poster_path.clone(),
                })
                .collect(),
        })
        .into_response(),
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_person_credits failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

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
        &state.app_ctx,
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
                movie: movie_to_dto(&w.movie),
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
    add_to_watchlist::execute(
        &state.app_ctx,
        AddToWatchlistCommand {
            user_id: user.0.value(),
            input: MovieInput {
                movie_id: Some(req.movie_id),
                external_metadata_id: None,
                manual_title: None,
                manual_release_year: None,
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
        &state.app_ctx,
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
        &state.app_ctx,
        IsOnWatchlistQuery {
            user_id: user.0.value(),
            movie_id,
        },
    )
    .await?;
    Ok(Json(WatchlistStatusResponse { on_watchlist }))
}
