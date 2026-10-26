use crate::extractors::{Params, PathId};
use crate::state::AppState;
use api_types::search::{
    CastCreditDto, CrewCreditDto, MovieSearchHitDto, PaginatedMovieHits, PaginatedPersonHits,
    PersonCreditsDto, PersonSearchHitDto, SearchQueryParams, SearchResponse,
};
use application::{
    person::{get as get_person, get_credits as get_person_credits},
    search::execute as search_uc,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use domain::models::{PersonId, collections::PageParams};

#[utoipa::path(
    get, path = "/api/v1/search",
    params(api_types::search::SearchQueryParams),
    responses(
        (status = 200, body = api_types::search::SearchResponse),
        (status = 400, description = "Invalid query", body = api_types::ErrorResponse),
    ),
    tag = "search",
)]
pub async fn get_search(
    State(state): State<AppState>,
    Params(params): Params<SearchQueryParams>,
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

    match search_uc::execute(&state.app_ctx.deps.search.execute, query).await {
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
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

#[utoipa::path(
    get, path = "/api/v1/people/{id}",
    params(("id" = Uuid, Path, description = "Person ID")),
    responses(
        (status = 200, body = api_types::search::PersonDto),
        (status = 404, description = "Person not found", body = api_types::ErrorResponse),
    ),
    tag = "search",
)]
pub async fn get_person_handler(
    State(state): State<AppState>,
    PathId(id): PathId<uuid::Uuid>,
) -> impl IntoResponse {
    match get_person::execute(
        &state.app_ctx.deps.person.get_person,
        PersonId::from_uuid(id),
    )
    .await
    {
        Ok(Some(person)) => axum::Json(api_types::mappers::person_to_dto(&person)).into_response(),
        Ok(None) => {
            crate::errors::refuse(StatusCode::NOT_FOUND, crate::errors::NOT_FOUND, "not found")
        }
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

#[utoipa::path(
    get, path = "/api/v1/people/{id}/credits",
    params(("id" = Uuid, Path, description = "Person ID")),
    responses(
        (status = 200, body = api_types::search::PersonCreditsDto),
        (status = 404, description = "Person not found", body = api_types::ErrorResponse),
    ),
    tag = "search",
)]
pub async fn get_person_credits_handler(
    State(state): State<AppState>,
    PathId(id): PathId<uuid::Uuid>,
) -> impl IntoResponse {
    match get_person_credits::execute(
        &state.app_ctx.deps.person.get_person,
        PersonId::from_uuid(id),
    )
    .await
    {
        Ok(credits) => axum::Json(PersonCreditsDto {
            person: api_types::mappers::person_to_dto(&credits.person),
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
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

pub async fn post_reindex_search(
    State(state): State<AppState>,
    _admin: crate::extractors::AdminApiUser,
) -> impl IntoResponse {
    match application::search::request_reindex::execute(&state.app_ctx.deps.search.request_reindex)
        .await
    {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(e) => {
            tracing::error!("failed to publish reindex event: {:?}", e);
            crate::errors::refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                crate::errors::INTERNAL,
                "internal server error",
            )
        }
    }
}
