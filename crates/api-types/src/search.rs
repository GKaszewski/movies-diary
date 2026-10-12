use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub use crate::common::{PaginatedMovieHits, PaginatedPersonHits};

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQueryParams {
    pub q: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u16>,
    pub person_id: Option<Uuid>,
    pub department: Option<String>,
    pub language: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    pub movies: PaginatedMovieHits,
    pub people: PaginatedPersonHits,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MovieSearchHitDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PersonSearchHitDto {
    pub person_id: Uuid,
    pub name: String,
    pub known_for_department: Option<String>,
    pub profile_path: Option<String>,
    pub known_for_titles: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PersonDto {
    pub id: Uuid,
    pub external_id: String,
    pub name: String,
    pub known_for_department: Option<String>,
    pub profile_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biography: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deathday: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_of_birth: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub also_known_as: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imdb_url: Option<String>,
    pub enriched: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PersonCreditsDto {
    pub person: PersonDto,
    pub cast: Vec<CastCreditDto>,
    pub crew: Vec<CrewCreditDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CastCreditDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub character: String,
    pub poster_path: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CrewCreditDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub job: String,
    pub department: String,
    pub poster_path: Option<String>,
}
