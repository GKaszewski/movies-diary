use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub movies: PaginatedMovieHits,
    pub people: PaginatedPersonHits,
}

#[derive(Debug, Serialize)]
pub struct PaginatedMovieHits {
    pub items: Vec<MovieSearchHitDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize)]
pub struct PaginatedPersonHits {
    pub items: Vec<PersonSearchHitDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize)]
pub struct MovieSearchHitDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PersonSearchHitDto {
    pub person_id: Uuid,
    pub name: String,
    pub known_for_department: Option<String>,
    pub profile_path: Option<String>,
    pub known_for_titles: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PersonDto {
    pub id: Uuid,
    pub external_id: String,
    pub name: String,
    pub known_for_department: Option<String>,
    pub profile_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PersonCreditsDto {
    pub person: PersonDto,
    pub cast: Vec<CastCreditDto>,
    pub crew: Vec<CrewCreditDto>,
}

#[derive(Debug, Serialize)]
pub struct CastCreditDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub character: String,
    pub poster_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrewCreditDto {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: Option<u16>,
    pub job: String,
    pub department: String,
    pub poster_path: Option<String>,
}
