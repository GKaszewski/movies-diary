use domain::models::{ExternalPersonId, Person, PersonId};

#[derive(sqlx::FromRow)]
pub(super) struct PersonRow {
    pub(super) id: String,
    pub(super) external_id: String,
    pub(super) name: String,
    pub(super) known_for_department: Option<String>,
    pub(super) profile_path: Option<String>,
    pub(super) biography: Option<String>,
    pub(super) birthday: Option<String>,
    pub(super) deathday: Option<String>,
    pub(super) place_of_birth: Option<String>,
    pub(super) also_known_as: Option<String>,
    pub(super) homepage: Option<String>,
    pub(super) imdb_id: Option<String>,
    pub(super) enriched_at: Option<String>,
}

impl PersonRow {
    pub(super) fn into_person(self) -> Person {
        let ext = ExternalPersonId::new(self.external_id);
        let also_known_as = self
            .also_known_as
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default();
        let birthday = self
            .birthday
            .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
        let deathday = self
            .deathday
            .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
        let enriched_at = self
            .enriched_at
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));
        Person::new(
            PersonId::from_uuid(uuid::Uuid::parse_str(&self.id).unwrap_or_default()),
            ext,
            self.name,
            self.known_for_department,
            self.profile_path,
            self.biography,
            birthday,
            deathday,
            self.place_of_birth,
            also_known_as,
            self.homepage,
            self.imdb_id,
            enriched_at,
        )
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct CastRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) release_year: Option<i64>,
    pub(super) character: String,
    pub(super) poster_path: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(super) struct CrewRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) release_year: Option<i64>,
    pub(super) job: String,
    pub(super) department: String,
    pub(super) poster_path: Option<String>,
}
