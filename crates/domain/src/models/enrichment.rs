use chrono::{DateTime, Utc};

use crate::value_objects::MovieId;

#[derive(Clone, Debug)]
pub struct Genre {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Keyword {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct CastMember {
    pub tmdb_person_id: u64,
    pub name: String,
    pub character: String,
    pub billing_order: u32,
    pub profile_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CrewMember {
    pub tmdb_person_id: u64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MovieProfile {
    pub movie_id: MovieId,
    pub tmdb_id: u64,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub budget_usd: Option<i64>,
    pub revenue_usd: Option<i64>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u32>,
    pub original_language: Option<String>,
    pub collection_name: Option<String>,
    pub genres: Vec<Genre>,
    pub keywords: Vec<Keyword>,
    pub cast: Vec<CastMember>,
    pub crew: Vec<CrewMember>,
    pub enriched_at: DateTime<Utc>,
}
