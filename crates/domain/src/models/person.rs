use uuid::Uuid;

use crate::models::MovieId;

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
mod tests {
    use super::*;

    #[test]
    fn person_new() {
        let ext = ExternalPersonId::new("tmdb:12345");
        let pid = PersonId::from_external(&ext);
        let p = Person::new(
            pid,
            ext,
            "Keanu Reeves".into(),
            Some("Acting".into()),
            Some("/profiles/keanu.jpg".into()),
        );
        assert_eq!(p.name(), "Keanu Reeves");
        assert_eq!(p.known_for_department(), Some("Acting"));
        assert_eq!(p.profile_path(), Some("/profiles/keanu.jpg"));
        assert_eq!(p.external_id().value(), "tmdb:12345");
        assert_eq!(p.external_id().tmdb_id(), Some(12345));
    }

    #[test]
    fn person_id_from_external() {
        let ext = ExternalPersonId::new("tmdb:99999");
        let pid = PersonId::from_external(&ext);
        // UUIDv5 is deterministic — just ensure it's a valid uuid
        let _ = pid.value();
    }

    #[test]
    fn person_id_deterministic() {
        let ext = ExternalPersonId::new("tmdb:42");
        let a = PersonId::from_external(&ext);
        let b = PersonId::from_external(&ext);
        assert_eq!(a, b);
    }

    #[test]
    fn person_credits_default_empty() {
        let ext = ExternalPersonId::new("tmdb:1");
        let pid = PersonId::from_external(&ext);
        let p = Person::new(pid, ext, "Test".into(), None, None);
        let credits = PersonCredits {
            person: p,
            cast: vec![],
            crew: vec![],
        };
        assert!(credits.cast.is_empty());
        assert!(credits.crew.is_empty());
    }

    #[test]
    fn external_person_id_tmdb_id_none_for_other() {
        let ext = ExternalPersonId::new("imdb:nm0000206");
        assert_eq!(ext.tmdb_id(), None);
    }
}
