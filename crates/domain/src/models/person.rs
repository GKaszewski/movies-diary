use crate::value_objects::{MovieId, PersonId};

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalPersonId(String);

impl ExternalPersonId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    /// Parse the TMDb numeric ID from "tmdb:12345". Returns None for other formats.
    pub fn tmdb_id(&self) -> Option<i64> {
        self.0.strip_prefix("tmdb:").and_then(|s| s.parse().ok())
    }
}

#[derive(Clone, Debug)]
pub struct Person {
    id: PersonId,
    external_id: ExternalPersonId,
    name: String,
    known_for_department: Option<String>,
    profile_path: Option<String>,
    biography: Option<String>,
    birthday: Option<chrono::NaiveDate>,
    deathday: Option<chrono::NaiveDate>,
    place_of_birth: Option<String>,
    also_known_as: Vec<String>,
    homepage: Option<String>,
    imdb_id: Option<String>,
    enriched_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Person {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PersonId,
        external_id: ExternalPersonId,
        name: String,
        known_for_department: Option<String>,
        profile_path: Option<String>,
        biography: Option<String>,
        birthday: Option<chrono::NaiveDate>,
        deathday: Option<chrono::NaiveDate>,
        place_of_birth: Option<String>,
        also_known_as: Vec<String>,
        homepage: Option<String>,
        imdb_id: Option<String>,
        enriched_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        Self {
            id,
            external_id,
            name,
            known_for_department,
            profile_path,
            biography,
            birthday,
            deathday,
            place_of_birth,
            also_known_as,
            homepage,
            imdb_id,
            enriched_at,
        }
    }

    pub fn basic(
        id: PersonId,
        external_id: ExternalPersonId,
        name: String,
        known_for_department: Option<String>,
        profile_path: Option<String>,
    ) -> Self {
        Self {
            id,
            external_id,
            name,
            known_for_department,
            profile_path,
            biography: None,
            birthday: None,
            deathday: None,
            place_of_birth: None,
            also_known_as: vec![],
            homepage: None,
            imdb_id: None,
            enriched_at: None,
        }
    }

    pub fn id(&self) -> &PersonId {
        &self.id
    }

    pub fn external_id(&self) -> &ExternalPersonId {
        &self.external_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn known_for_department(&self) -> Option<&str> {
        self.known_for_department.as_deref()
    }

    pub fn profile_path(&self) -> Option<&str> {
        self.profile_path.as_deref()
    }

    pub fn biography(&self) -> Option<&str> {
        self.biography.as_deref()
    }

    pub fn birthday(&self) -> Option<chrono::NaiveDate> {
        self.birthday
    }

    pub fn deathday(&self) -> Option<chrono::NaiveDate> {
        self.deathday
    }

    pub fn place_of_birth(&self) -> Option<&str> {
        self.place_of_birth.as_deref()
    }

    pub fn also_known_as(&self) -> &[String] {
        &self.also_known_as
    }

    pub fn homepage(&self) -> Option<&str> {
        self.homepage.as_deref()
    }

    pub fn imdb_id(&self) -> Option<&str> {
        self.imdb_id.as_deref()
    }

    pub fn enriched_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.enriched_at
    }
}

#[derive(Clone, Debug)]
pub struct PersonEnrichmentData {
    pub biography: Option<String>,
    pub birthday: Option<chrono::NaiveDate>,
    pub deathday: Option<chrono::NaiveDate>,
    pub place_of_birth: Option<String>,
    pub also_known_as: Vec<String>,
    pub homepage: Option<String>,
    pub imdb_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PersonCredits {
    pub person: Person,
    pub cast: Vec<CastCredit>,
    pub crew: Vec<CrewCredit>,
}

#[derive(Clone, Debug)]
pub struct CastCredit {
    pub movie_id: MovieId,
    pub title: String,
    pub release_year: Option<u16>,
    pub character: String,
    pub poster_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CrewCredit {
    pub movie_id: MovieId,
    pub title: String,
    pub release_year: Option<u16>,
    pub job: String,
    pub department: String,
    pub poster_path: Option<String>,
}

#[cfg(test)]
#[path = "tests/person.rs"]
mod tests;
