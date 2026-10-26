use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{ExternalPersonId, Person, PersonEnrichmentData, PersonId},
    ports::PersonCommand,
};

use super::SqlitePersonAdapter;

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
            .map_err(crate::shared::map_sqlx_error)?;
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
        .map_err(crate::shared::map_sqlx_error)?;

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
            .map_err(crate::shared::map_sqlx_error)?;
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
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE persons SET biography = ?, birthday = ?, deathday = ?, place_of_birth = ?, also_known_as = ?, homepage = ?, imdb_id = ?, enriched_at = ? WHERE id = ?",
        )
        .bind(&data.biography)
        .bind(data.birthday.map(|d| d.to_string()))
        .bind(data.deathday.map(|d| d.to_string()))
        .bind(&data.place_of_birth)
        .bind(&also_known_as_json)
        .bind(&data.homepage)
        .bind(&data.imdb_id)
        .bind(&now)
        .bind(id.value().to_string())
        .execute(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        Ok(())
    }
}
