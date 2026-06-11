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
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct SqlitePersonAdapter {
    pool: SqlitePool,
}

impl SqlitePersonAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn create_person_adapter(pool: SqlitePool) -> (Arc<dyn PersonCommand>, Arc<dyn PersonQuery>) {
    let adapter = Arc::new(SqlitePersonAdapter::new(pool));
    (
        Arc::clone(&adapter) as Arc<dyn PersonCommand>,
        adapter as Arc<dyn PersonQuery>,
    )
}

fn map_err(e: sqlx::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl PersonCommand for SqlitePersonAdapter {
    async fn upsert_batch(&self, persons: &[Person]) -> Result<(), DomainError> {
        for person in persons {
            let tmdb_id = person.external_id().tmdb_id();
            sqlx::query(
                "INSERT INTO persons (id, external_id, tmdb_person_id, name, known_for_department, profile_path)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                   external_id = excluded.external_id,
                   tmdb_person_id = excluded.tmdb_person_id,
                   name = excluded.name,
                   known_for_department = excluded.known_for_department,
                   profile_path = excluded.profile_path",
            )
            .bind(person.id().value().to_string())
            .bind(person.external_id().value())
            .bind(tmdb_id)
            .bind(person.name())
            .bind(person.known_for_department())
            .bind(person.profile_path())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
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
             GROUP BY mc.tmdb_person_id
             UNION ALL
             SELECT mc.tmdb_person_id, mc.name, mc.department, mc.profile_path
             FROM movie_crew mc
             WHERE NOT EXISTS (SELECT 1 FROM persons WHERE persons.tmdb_person_id = mc.tmdb_person_id)
               AND NOT EXISTS (SELECT 1 FROM movie_cast c2 WHERE c2.tmdb_person_id = mc.tmdb_person_id)
             GROUP BY mc.tmdb_person_id
             LIMIT ?",
        )
        .bind(batch_size)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let has_more = rows.len() as u32 >= batch_size;
        let mut count = 0u64;
        for row in &rows {
            let ext = ExternalPersonId::new(format!("tmdb:{}", row.tmdb_person_id));
            let pid = PersonId::from_external(&ext);
            sqlx::query(
                "INSERT INTO persons (id, external_id, tmdb_person_id, name, known_for_department, profile_path)
                 VALUES (?, ?, ?, ?, ?, ?)
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
            .map_err(map_err)?;
            count += 1;
        }
        Ok((count, has_more))
    }

    async fn update_enrichment(
        &self,
        _id: &PersonId,
        _data: &PersonEnrichmentData,
    ) -> Result<(), DomainError> {
        todo!("person enrichment persistence")
    }
}

#[async_trait]
impl PersonQuery for SqlitePersonAdapter {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError> {
        let row = sqlx::query_as::<_, PersonRow>(
            "SELECT id, external_id, name, known_for_department, profile_path FROM persons WHERE id = ?",
        )
        .bind(id.value().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(PersonRow::into_person))
    }

    async fn get_by_external_id(
        &self,
        id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        let row = sqlx::query_as::<_, PersonRow>(
            "SELECT id, external_id, name, known_for_department, profile_path FROM persons WHERE external_id = ?",
        )
        .bind(id.value())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(PersonRow::into_person))
    }

    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError> {
        let person = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Person {} not found", id.value())))?;

        let tmdb_id: Option<i64> =
            sqlx::query_scalar("SELECT tmdb_person_id FROM persons WHERE id = ?")
                .bind(id.value().to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?
                .flatten();

        let Some(tmdb_id) = tmdb_id else {
            return Ok(PersonCredits {
                person,
                cast: vec![],
                crew: vec![],
            });
        };

        let cast = sqlx::query_as::<_, CastRow>(
            "SELECT m.id, m.title, m.release_year, mc.character, m.poster_path
             FROM movie_cast mc
             JOIN movies m ON m.id = mc.movie_id
             WHERE mc.tmdb_person_id = ?
             ORDER BY mc.billing_order",
        )
        .bind(tmdb_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
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
             FROM movie_crew mc
             JOIN movies m ON m.id = mc.movie_id
             WHERE mc.tmdb_person_id = ?
             ORDER BY m.title",
        )
        .bind(tmdb_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
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
            "SELECT id, external_id, name, known_for_department, profile_path FROM persons ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

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
        .map_err(map_err)?;

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
}

impl PersonRow {
    fn into_person(self) -> Person {
        let ext = ExternalPersonId::new(self.external_id);
        Person::basic(
            PersonId::from_uuid(uuid::Uuid::parse_str(&self.id).unwrap_or_default()),
            ext,
            self.name,
            self.known_for_department,
            self.profile_path,
        )
    }
}

#[derive(sqlx::FromRow)]
struct CastRow {
    id: String,
    title: String,
    release_year: Option<i64>,
    character: String,
    poster_path: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CrewRow {
    id: String,
    title: String,
    release_year: Option<i64>,
    job: String,
    department: String,
    poster_path: Option<String>,
}

#[cfg(test)]
#[path = "tests/persons.rs"]
mod tests;
