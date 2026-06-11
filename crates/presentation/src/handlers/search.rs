use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use application::{
    person::{get as get_person, get_credits as get_person_credits},
    search::execute as search_uc,
};
use domain::models::{PersonId, collections::PageParams};

use crate::state::AppState;
use api_types::search::{
    CastCreditDto, CrewCreditDto, MovieSearchHitDto, PaginatedMovieHits, PaginatedPersonHits,
    PersonCreditsDto, PersonDto, PersonSearchHitDto, SearchQueryParams, SearchResponse,
};

// ── API ──────────────────────────────────────────────────────────────────────

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

    match search_uc::execute(state.app_ctx.repos.search_port.clone(), query).await {
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
        Err(e) => crate::errors::domain_error_response(e),
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
            biography: person.biography().map(str::to_string),
            birthday: person.birthday().map(|d| d.to_string()),
            deathday: person.deathday().map(|d| d.to_string()),
            place_of_birth: person.place_of_birth().map(str::to_string),
            also_known_as: person.also_known_as().to_vec(),
            homepage: person.homepage().map(str::to_string),
            imdb_url: person
                .imdb_id()
                .map(|id| format!("https://www.imdb.com/name/{id}")),
            enriched: person.enriched_at().is_some(),
        })
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
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
                biography: credits.person.biography().map(str::to_string),
                birthday: credits.person.birthday().map(|d| d.to_string()),
                deathday: credits.person.deathday().map(|d| d.to_string()),
                place_of_birth: credits.person.place_of_birth().map(str::to_string),
                also_known_as: credits.person.also_known_as().to_vec(),
                homepage: credits.person.homepage().map(str::to_string),
                imdb_url: credits
                    .person
                    .imdb_id()
                    .map(|id| format!("https://www.imdb.com/name/{id}")),
                enriched: credits.person.enriched_at().is_some(),
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
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn post_reindex_search(
    State(state): State<AppState>,
    _admin: crate::extractors::AdminApiUser,
) -> impl IntoResponse {
    let event = domain::events::DomainEvent::SearchReindexRequested;
    match state.app_ctx.services.event_publisher.publish(&event).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(e) => {
            tracing::error!("failed to publish reindex event: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_tag(Path(tag): Path<String>) -> impl IntoResponse {
    if tag.eq_ignore_ascii_case("moviesdiary") {
        Redirect::temporary("/")
    } else {
        Redirect::temporary(&format!("/?search={}", tag))
    }
}
