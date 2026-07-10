use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        CastCredit, CrewCredit, ExternalPersonId, Person, PersonCredits, PersonEnrichmentData,
        PersonId,
    },
    ports::{PersonCommand, PersonQuery},
    value_objects::MovieId,
};
use sqlx::PgPool;
use std::sync::Arc;

pub struct PostgresPersonAdapter {
    pool: PgPool,
}

impl PostgresPersonAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub fn create_person_adapter(pool: PgPool) -> (Arc<dyn PersonCommand>, Arc<dyn PersonQuery>) {
    let adapter = Arc::new(PostgresPersonAdapter::new(pool));
    (
        Arc::clone(&adapter) as Arc<dyn PersonCommand>,
        adapter as Arc<dyn PersonQuery>,
    )
}

#[async_trait]
impl PersonCommand for PostgresPersonAdapter {
    async fn upsert_batch(&self, persons: &[Person]) -> Result<(), DomainError> {
        for person in persons {
            let tmdb_id = person.external_id().tmdb_id();
            sqlx::query(
                "INSERT INTO persons (id, external_id, tmdb_person_id, name, known_for_department, profile_path)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT(id) DO UPDATE SET
                   external_id = EXCLUDED.external_id,
                   tmdb_person_id = EXCLUDED.tmdb_person_id,
                   name = EXCLUDED.name,
                   known_for_department = EXCLUDED.known_for_department,
                   profile_path = EXCLUDED.profile_path",
            )
            .bind(person.id().value().to_string())
            .bind(person.external_id().value())
            .bind(tmdb_id)
            .bind(person.name())
            .bind(person.known_for_department())
            .bind(person.profile_path())
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        }
        Ok(())
    }

    async fn backfill_from_credits_batch(
        &self,
        batch_size: u32,
    ) -> Result<(u64, bool), DomainError> {
        #[derive(sqlx::FromRow)]
        struct MissingPerson {
            tmdb_person_id: i64,
            name: String,
            department: Option<String>,
            profile_path: Option<String>,
        }

        let rows = sqlx::query_as::<_, MissingPerson>(
            "SELECT mc.tmdb_person_id, mc.name, 'Acting' AS department, mc.profile_path
             FROM movie_cast mc
             WHERE NOT EXISTS (SELECT 1 FROM persons WHERE persons.tmdb_person_id = mc.tmdb_person_id)
             GROUP BY mc.tmdb_person_id, mc.name, mc.profile_path
             UNION ALL
             SELECT mc.tmdb_person_id, mc.name, mc.department, mc.profile_path
             FROM movie_crew mc
             WHERE NOT EXISTS (SELECT 1 FROM persons WHERE persons.tmdb_person_id = mc.tmdb_person_id)
               AND NOT EXISTS (SELECT 1 FROM movie_cast c2 WHERE c2.tmdb_person_id = mc.tmdb_person_id)
             GROUP BY mc.tmdb_person_id, mc.name, mc.department, mc.profile_path
             LIMIT $1",
        )
        .bind(batch_size as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        let has_more = rows.len() as u32 >= batch_size;
        let mut count = 0u64;
        for row in &rows {
            let ext = ExternalPersonId::new(format!("tmdb:{}", row.tmdb_person_id));
            let pid = PersonId::from_external(&ext);
            sqlx::query(
                "INSERT INTO persons (id, external_id, tmdb_person_id, name, known_for_department, profile_path)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT(tmdb_person_id) DO NOTHING",
            )
            .bind(pid.value().to_string())
            .bind(ext.value())
            .bind(row.tmdb_person_id)
            .bind(&row.name)
            .bind(&row.department)
            .bind(&row.profile_path)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
            count += 1;
        }
        Ok((count, has_more))
    }

    async fn update_enrichment(
        &self,
        id: &PersonId,
        data: &PersonEnrichmentData,
    ) -> Result<(), DomainError> {
        let also_known_as_json =
            serde_json::to_string(&data.also_known_as).unwrap_or_else(|_| "[]".into());
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE persons SET biography = $1, birthday = $2, deathday = $3, place_of_birth = $4, also_known_as = $5, homepage = $6, imdb_id = $7, enriched_at = $8 WHERE id = $9",
        )
        .bind(&data.biography)
        .bind(data.birthday.map(|d| d.to_string()))
        .bind(data.deathday.map(|d| d.to_string()))
        .bind(&data.place_of_birth)
        .bind(&also_known_as_json)
        .bind(&data.homepage)
        .bind(&data.imdb_id)
        .bind(now)
        .bind(id.value().to_string())
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        Ok(())
    }
}

#[async_trait]
impl PersonQuery for PostgresPersonAdapter {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError> {
        let row = sqlx::query_as::<_, PersonRow>(
            "SELECT id, external_id, name, known_for_department, profile_path, biography, birthday, deathday, place_of_birth, also_known_as, homepage, imdb_id, enriched_at FROM persons WHERE id = $1",
        )
        .bind(id.value().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(row.map(PersonRow::into_person))
    }

    async fn get_by_external_id(
        &self,
        id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        let row = sqlx::query_as::<_, PersonRow>(
            "SELECT id, external_id, name, known_for_department, profile_path, biography, birthday, deathday, place_of_birth, also_known_as, homepage, imdb_id, enriched_at FROM persons WHERE external_id = $1",
        )
        .bind(id.value())
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(row.map(PersonRow::into_person))
    }

    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError> {
        let person = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Person {} not found", id.value())))?;

        let tmdb_id: Option<i64> =
            sqlx::query_scalar("SELECT tmdb_person_id FROM persons WHERE id = $1")
                .bind(id.value().to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(adapter_common::map_sqlx_error)?
                .flatten();

        let Some(tmdb_id) = tmdb_id else {
            return Ok(PersonCredits {
                person,
                cast: vec![],
                crew: vec![],
            });
        };

        #[derive(sqlx::FromRow)]
        struct CastRow {
            id: String,
            title: String,
            release_year: Option<i32>,
            character: String,
            poster_path: Option<String>,
        }
        #[derive(sqlx::FromRow)]
        struct CrewRow {
            id: String,
            title: String,
            release_year: Option<i32>,
            job: String,
            department: String,
            poster_path: Option<String>,
        }

        let cast = sqlx::query_as::<_, CastRow>(
            "SELECT m.id, m.title, m.release_year, mc.character, m.poster_path
             FROM movie_cast mc JOIN movies m ON m.id = mc.movie_id
             WHERE mc.tmdb_person_id = $1 ORDER BY mc.billing_order",
        )
        .bind(tmdb_id)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(|r| CastCredit {
            movie_id: MovieId::from_uuid(uuid::Uuid::parse_str(&r.id).unwrap_or_default()),
            title: r.title,
            release_year: r.release_year.map(|y| y as u16),
            character: r.character,
            poster_path: r.poster_path,
        })
        .collect();

        let crew = sqlx::query_as::<_, CrewRow>(
            "SELECT m.id, m.title, m.release_year, mc.job, mc.department, m.poster_path
             FROM movie_crew mc JOIN movies m ON m.id = mc.movie_id
             WHERE mc.tmdb_person_id = $1 ORDER BY m.title",
        )
        .bind(tmdb_id)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(|r| CrewCredit {
            movie_id: MovieId::from_uuid(uuid::Uuid::parse_str(&r.id).unwrap_or_default()),
            title: r.title,
            release_year: r.release_year.map(|y| y as u16),
            job: r.job,
            department: r.department,
            poster_path: r.poster_path,
        })
        .collect();

        Ok(PersonCredits { person, cast, crew })
    }

    async fn list_page(&self, limit: u32, offset: u32) -> Result<Vec<Person>, DomainError> {
        let rows = sqlx::query_as::<_, PersonRow>(
            "SELECT id, external_id, name, known_for_department, profile_path, biography, birthday, deathday, place_of_birth, also_known_as, homepage, imdb_id, enriched_at FROM persons ORDER BY id LIMIT $1 OFFSET $2",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(rows.into_iter().map(PersonRow::into_person).collect())
    }

    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM persons
             WHERE NOT EXISTS (
                 SELECT 1 FROM movie_cast WHERE movie_cast.tmdb_person_id = persons.tmdb_person_id
             )
             AND NOT EXISTS (
                 SELECT 1 FROM movie_crew WHERE movie_crew.tmdb_person_id = persons.tmdb_person_id
             )
             LIMIT 500",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|(id,)| uuid::Uuid::parse_str(&id).ok().map(PersonId::from_uuid))
            .collect())
    }
}

// ── Row types ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct PersonRow {
    id: String,
    external_id: String,
    name: String,
    known_for_department: Option<String>,
    profile_path: Option<String>,
    biography: Option<String>,
    birthday: Option<String>,
    deathday: Option<String>,
    place_of_birth: Option<String>,
    also_known_as: Option<String>,
    homepage: Option<String>,
    imdb_id: Option<String>,
    enriched_at: Option<String>,
}

impl PersonRow {
    fn into_person(self) -> Person {
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
