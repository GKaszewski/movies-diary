use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    errors::DomainError,
    models::{CastMember, CrewMember, Genre, Keyword, MovieProfile},
    ports::MovieProfileRepository,
    value_objects::MovieId,
};
use sqlx::{PgPool, Row};

pub struct PostgresMovieProfileRepository {
    pool: PgPool,
}

impl PostgresMovieProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MovieProfileRepository for PostgresMovieProfileRepository {
    async fn upsert(&self, p: &MovieProfile) -> Result<(), DomainError> {
        let movie_id = p.movie_id.value().to_string();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        sqlx::query(
            r#"INSERT INTO movie_profiles
               (movie_id, tmdb_id, imdb_id, overview, tagline, runtime_minutes,
                budget_usd, revenue_usd, vote_average, vote_count,
                original_language, collection_name, enriched_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
               ON CONFLICT(movie_id) DO UPDATE SET
                 tmdb_id=EXCLUDED.tmdb_id, imdb_id=EXCLUDED.imdb_id,
                 overview=EXCLUDED.overview, tagline=EXCLUDED.tagline,
                 runtime_minutes=EXCLUDED.runtime_minutes,
                 budget_usd=EXCLUDED.budget_usd, revenue_usd=EXCLUDED.revenue_usd,
                 vote_average=EXCLUDED.vote_average, vote_count=EXCLUDED.vote_count,
                 original_language=EXCLUDED.original_language,
                 collection_name=EXCLUDED.collection_name,
                 enriched_at=EXCLUDED.enriched_at"#,
        )
        .bind(&movie_id)
        .bind(p.tmdb_id as i64)
        .bind(&p.imdb_id)
        .bind(&p.overview)
        .bind(&p.tagline)
        .bind(p.runtime_minutes.map(|v| v as i32))
        .bind(p.budget_usd)
        .bind(p.revenue_usd)
        .bind(p.vote_average)
        .bind(p.vote_count.map(|v| v as i32))
        .bind(&p.original_language)
        .bind(&p.collection_name)
        .bind(p.enriched_at)
        .execute(&mut *tx)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        sqlx::query("DELETE FROM movie_genres WHERE movie_id = $1")
            .bind(&movie_id)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        for g in &p.genres {
            sqlx::query("INSERT INTO movie_genres (movie_id, tmdb_id, name) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(&movie_id).bind(g.tmdb_id as i32).bind(&g.name)
                .execute(&mut *tx).await.map_err(adapter_common::map_sqlx_error)?;
        }

        sqlx::query("DELETE FROM movie_keywords WHERE movie_id = $1")
            .bind(&movie_id)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        for k in &p.keywords {
            sqlx::query("INSERT INTO movie_keywords (movie_id, tmdb_id, name) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(&movie_id).bind(k.tmdb_id as i32).bind(&k.name)
                .execute(&mut *tx).await.map_err(adapter_common::map_sqlx_error)?;
        }

        sqlx::query("DELETE FROM movie_cast WHERE movie_id = $1")
            .bind(&movie_id)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        for c in &p.cast {
            sqlx::query(
                "INSERT INTO movie_cast \
                 (movie_id, tmdb_person_id, name, character, billing_order, profile_path) \
                 VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
            )
            .bind(&movie_id)
            .bind(c.tmdb_person_id as i64)
            .bind(&c.name)
            .bind(&c.character)
            .bind(c.billing_order as i32)
            .bind(&c.profile_path)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        }

        sqlx::query("DELETE FROM movie_crew WHERE movie_id = $1")
            .bind(&movie_id)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        for cr in &p.crew {
            sqlx::query(
                "INSERT INTO movie_crew \
                 (movie_id, tmdb_person_id, name, job, department, profile_path) \
                 VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
            )
            .bind(&movie_id)
            .bind(cr.tmdb_person_id as i64)
            .bind(&cr.name)
            .bind(&cr.job)
            .bind(&cr.department)
            .bind(&cr.profile_path)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        }

        tx.commit().await.map_err(adapter_common::map_sqlx_error)
    }

    async fn get_by_movie_id(&self, id: &MovieId) -> Result<Option<MovieProfile>, DomainError> {
        let movie_id = id.value().to_string();

        let row = sqlx::query(
            "SELECT tmdb_id, imdb_id, overview, tagline, runtime_minutes, budget_usd,
                    revenue_usd, vote_average, vote_count, original_language,
                    collection_name, enriched_at
             FROM movie_profiles WHERE movie_id = $1",
        )
        .bind(&movie_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let enriched_at: DateTime<Utc> = row
            .try_get("enriched_at")
            .map_err(|_| DomainError::InfrastructureError("invalid enriched_at".into()))?;

        let genres = sqlx::query("SELECT tmdb_id, name FROM movie_genres WHERE movie_id = $1")
            .bind(&movie_id)
            .fetch_all(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .into_iter()
            .map(|r| Genre {
                tmdb_id: r.try_get::<i32, _>("tmdb_id").unwrap_or(0) as u32,
                name: r.try_get("name").unwrap_or_default(),
            })
            .collect();

        let keywords = sqlx::query("SELECT tmdb_id, name FROM movie_keywords WHERE movie_id = $1")
            .bind(&movie_id)
            .fetch_all(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .into_iter()
            .map(|r| Keyword {
                tmdb_id: r.try_get::<i32, _>("tmdb_id").unwrap_or(0) as u32,
                name: r.try_get("name").unwrap_or_default(),
            })
            .collect();

        let cast = sqlx::query(
            "SELECT tmdb_person_id, name, character, billing_order, profile_path \
             FROM movie_cast WHERE movie_id = $1 ORDER BY billing_order",
        )
        .bind(&movie_id)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(|r| CastMember {
            tmdb_person_id: r.try_get::<i64, _>("tmdb_person_id").unwrap_or(0) as u64,
            name: r.try_get("name").unwrap_or_default(),
            character: r.try_get("character").unwrap_or_default(),
            billing_order: r.try_get::<i32, _>("billing_order").unwrap_or(0) as u32,
            profile_path: r.try_get("profile_path").ok(),
        })
        .collect();

        let crew = sqlx::query(
            "SELECT tmdb_person_id, name, job, department, profile_path \
             FROM movie_crew WHERE movie_id = $1",
        )
        .bind(&movie_id)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(|r| CrewMember {
            tmdb_person_id: r.try_get::<i64, _>("tmdb_person_id").unwrap_or(0) as u64,
            name: r.try_get("name").unwrap_or_default(),
            job: r.try_get("job").unwrap_or_default(),
            department: r.try_get("department").unwrap_or_default(),
            profile_path: r.try_get("profile_path").ok(),
        })
        .collect();

        Ok(Some(MovieProfile {
            movie_id: id.clone(),
            tmdb_id: row.try_get::<i64, _>("tmdb_id").unwrap_or(0) as u64,
            imdb_id: row.try_get("imdb_id").ok(),
            overview: row.try_get("overview").ok(),
            tagline: row.try_get("tagline").ok(),
            runtime_minutes: row
                .try_get::<Option<i32>, _>("runtime_minutes")
                .ok()
                .flatten()
                .map(|v| v as u32),
            budget_usd: row.try_get("budget_usd").ok(),
            revenue_usd: row.try_get("revenue_usd").ok(),
            vote_average: row.try_get("vote_average").ok(),
            vote_count: row
                .try_get::<Option<i32>, _>("vote_count")
                .ok()
                .flatten()
                .map(|v| v as u32),
            original_language: row.try_get("original_language").ok(),
            collection_name: row.try_get("collection_name").ok(),
            genres,
            keywords,
            cast,
            crew,
            enriched_at,
        }))
    }

    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError> {
        let threshold = Utc::now() - chrono::Duration::days(30);
        let rows = sqlx::query(
            r#"SELECT m.id, m.external_metadata_id
               FROM movies m
               LEFT JOIN movie_profiles p ON p.movie_id = m.id
               WHERE m.external_metadata_id IS NOT NULL
                 AND (p.movie_id IS NULL OR p.enriched_at < $1)
               ORDER BY p.enriched_at ASC NULLS FIRST"#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ext_id: Option<String> = r.try_get("external_metadata_id").ok()?;
                let ext_id = ext_id?;
                let id_str: String = r.try_get("id").ok()?;
                let movie_id = id_str.parse::<uuid::Uuid>().ok().map(MovieId::from_uuid)?;
                Some((movie_id, ext_id))
            })
            .collect())
    }
}
