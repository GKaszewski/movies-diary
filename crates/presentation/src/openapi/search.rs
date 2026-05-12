use api_types::search::{
    CastCreditDto, CrewCreditDto, MovieSearchHitDto, PaginatedMovieHits, PaginatedPersonHits,
    PersonCreditsDto, PersonDto, PersonSearchHitDto, SearchResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::get_search,
        crate::handlers::api::get_person_handler,
        crate::handlers::api::get_person_credits_handler,
    ),
    components(schemas(
        SearchResponse,
        PaginatedMovieHits,
        PaginatedPersonHits,
        MovieSearchHitDto,
        PersonSearchHitDto,
        PersonDto,
        PersonCreditsDto,
        CastCreditDto,
        CrewCreditDto,
    )),
    tags(
        (name = "search", description = "Full-text search across movies and people"),
    ),
)]
pub struct SearchDoc;
