use uuid::Uuid;

use crate::value_objects::MovieId;

#[derive(Clone, Debug, PartialEq)]
pub struct PersonId(Uuid);

impl PersonId {
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Deterministic UUIDv5 from an external person ID string.
    /// "tmdb:12345" always maps to the same PersonId.
    pub fn from_external(external_id: &ExternalPersonId) -> Self {
        Self(Uuid::new_v5(&Uuid::NAMESPACE_URL, external_id.0.as_bytes()))
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

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
}

impl Person {
    pub fn new(
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
