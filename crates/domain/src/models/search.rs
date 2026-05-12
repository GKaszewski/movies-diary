use crate::models::{
    Movie, MovieId, MovieProfile, Person, PersonId,
    collections::{PageParams, Paginated},
};

#[derive(Clone, Debug, Default)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub filters: SearchFilters,
    pub page: PageParams,
}

#[derive(Clone, Debug, Default)]
pub struct SearchFilters {
    pub genre: Option<String>,
    pub year: Option<u16>,
    pub person_id: Option<PersonId>,
    pub department: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SearchResults {
    pub movies: Paginated<MovieSearchHit>,
    pub people: Paginated<PersonSearchHit>,
}

#[derive(Clone, Debug)]
pub struct MovieSearchHit {
    pub movie_id: MovieId,
    pub title: String,
    pub release_year: Option<u16>,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PersonSearchHit {
    pub person_id: PersonId,
    pub name: String,
    pub known_for_department: Option<String>,
    pub profile_path: Option<String>,
    /// Top movie titles this person is known for — populated at query time
    /// by joining relational tables, never from the index.
    pub known_for_titles: Vec<String>,
}

/// Document submitted to the search index.
/// Add a new variant here to make a new entity type searchable — the port never changes.
pub enum IndexableDocument {
    Movie {
        id: MovieId,
        movie: Box<Movie>,
        profile: Option<Box<MovieProfile>>,
    },
    Person {
        id: PersonId,
        person: Box<Person>,
        // known_for_titles intentionally absent — no reads inside a command flow
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityType {
    Movie,
    Person,
}
