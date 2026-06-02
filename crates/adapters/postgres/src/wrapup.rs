use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    models::wrapup::{DateRange, WrapUpRecord, WrapUpScope, WrapUpStatus},
    ports::{WrapUpMovieRow, WrapUpRepository, WrapUpStatsQuery},
    value_objects::WrapUpId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{parse_datetime, parse_uuid};

fn map_err(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database operation failed".into())
}

fn status_to_str(s: &WrapUpStatus) -> &'static str {
    match s {
        WrapUpStatus::Pending => "pending",
        WrapUpStatus::Generating => "generating",
        WrapUpStatus::Ready => "ready",
        WrapUpStatus::Failed => "failed",
    }
}

fn parse_status(s: &str) -> Result<WrapUpStatus, DomainError> {
    match s {
        "pending" => Ok(WrapUpStatus::Pending),
        "generating" => Ok(WrapUpStatus::Generating),
        "ready" => Ok(WrapUpStatus::Ready),
        "failed" => Ok(WrapUpStatus::Failed),
        other => Err(DomainError::InfrastructureError(format!(
            "Unknown wrap-up status: {other}"
        ))),
    }
}

// ── WrapUpRepository ─────────────────────────────────────────────────────────

pub struct PostgresWrapUpRepository {
    pool: PgPool,
}

impl PostgresWrapUpRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrapUpRepository for PostgresWrapUpRepository {
    async fn create(&self, record: &WrapUpRecord) -> Result<(), DomainError> {
        let id = record.id.value().to_string();
        let user_id = record.user_id.map(|u| u.to_string());
        let status = status_to_str(&record.status);

        sqlx::query(
            "INSERT INTO wrap_up_records \
             (id, user_id, start_date, end_date, status, report_json, error_message, created_at, completed_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(record.start_date)
        .bind(record.end_date)
        .bind(status)
        .bind(&record.report_json)
        .bind(&record.error_message)
        .bind(record.created_at)
        .bind(record.completed_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let status_str = status_to_str(status);

        sqlx::query("UPDATE wrap_up_records SET status = $1, error_message = $2 WHERE id = $3")
            .bind(status_str)
            .bind(error)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(())
    }

    async fn set_complete(&self, id: &WrapUpId, report_json: &str) -> Result<(), DomainError> {
        let id_str = id.value().to_string();

        sqlx::query(
            "UPDATE wrap_up_records \
             SET status = 'ready', report_json = $1, completed_at = NOW() \
             WHERE id = $2",
        )
        .bind(report_json)
        .bind(&id_str)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn get_by_id(&self, id: &WrapUpId) -> Result<Option<WrapUpRecord>, DomainError> {
        let id_str = id.value().to_string();

        let row = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS completed_at \
             FROM wrap_up_records WHERE id = $1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.as_ref().map(row_to_record).transpose()
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<WrapUpRecord>, DomainError> {
        let uid = user_id.to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS completed_at \
             FROM wrap_up_records WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.iter().map(row_to_record).collect()
    }

    async fn list_global(&self) -> Result<Vec<WrapUpRecord>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS completed_at \
             FROM wrap_up_records WHERE user_id IS NULL ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.iter().map(row_to_record).collect()
    }

    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<WrapUpRecord>, DomainError> {
        let uid = user_id.map(|u| u.to_string());

        let row = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS completed_at \
             FROM wrap_up_records \
             WHERE (($1::text IS NULL AND user_id IS NULL) OR user_id = $1) \
               AND start_date = $2 AND end_date = $3 \
             LIMIT 1",
        )
        .bind(&uid)
        .bind(start)
        .bind(end)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.as_ref().map(row_to_record).transpose()
    }

    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM wrap_up_records WHERE id = $1")
            .bind(id.value().to_string())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete_failed_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let result = sqlx::query(
            "DELETE FROM wrap_up_records WHERE status = 'failed' AND created_at < $1",
        )
        .bind(before)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result.rows_affected())
    }
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> Result<WrapUpRecord, DomainError> {
    let id_str: String = row.try_get("id").map_err(map_err)?;
    let user_id_str: Option<String> = row.try_get("user_id").map_err(map_err)?;
    let start_date: NaiveDate = row.try_get("start_date").map_err(map_err)?;
    let end_date: NaiveDate = row.try_get("end_date").map_err(map_err)?;
    let status_str: String = row.try_get("status").map_err(map_err)?;
    let report_json: Option<String> = row.try_get("report_json").map_err(map_err)?;
    let error_message: Option<String> = row.try_get("error_message").map_err(map_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_err)?;
    let completed_at_str: Option<String> = row.try_get("completed_at").map_err(map_err)?;

    let user_id = user_id_str.as_deref().map(parse_uuid).transpose()?;

    Ok(WrapUpRecord {
        id: WrapUpId::from_uuid(parse_uuid(&id_str)?),
        user_id,
        start_date,
        end_date,
        status: parse_status(&status_str)?,
        report_json,
        error_message,
        created_at: parse_datetime(&created_at_str)?,
        completed_at: completed_at_str
            .as_deref()
            .map(parse_datetime)
            .transpose()?,
    })
}

// ── WrapUpStatsQuery ─────────────────────────────────────────────────────────

pub struct PostgresWrapUpStatsQuery {
    pool: PgPool,
}

impl PostgresWrapUpStatsQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrapUpStatsQuery for PostgresWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        scope: &WrapUpScope,
        range: &DateRange,
    ) -> Result<Vec<WrapUpMovieRow>, DomainError> {
        // 1) Main query: reviews + movies + movie_profiles
        let (scope_clause, scope_bind) = match scope {
            WrapUpScope::User(uid) => ("AND r.user_id = $3", Some(uid.to_string())),
            WrapUpScope::Global => ("", None),
        };

        let sql = format!(
            "SELECT r.movie_id, m.title, m.release_year, m.director, m.poster_path, \
                    r.rating, \
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at, \
                    r.user_id, \
                    p.runtime_minutes, p.budget_usd, p.original_language \
             FROM reviews r \
             INNER JOIN movies m ON m.id = r.movie_id \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             WHERE r.watched_at >= $1 AND r.watched_at < $2 {scope_clause} \
             ORDER BY r.watched_at ASC"
        );

        let mut q = sqlx::query(&sql).bind(range.start).bind(range.end);
        if let Some(ref uid) = scope_bind {
            q = q.bind(uid);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // Collect unique movie IDs
        let mut movie_ids: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in &rows {
            let mid: String = row.try_get("movie_id").map_err(map_err)?;
            if seen.insert(mid.clone()) {
                movie_ids.push(mid);
            }
        }

        // 2) Batch-fetch genres, keywords, cast
        let (genres_map, keywords_map, cast_map) = tokio::try_join!(
            fetch_genres_pg(&self.pool, &movie_ids),
            fetch_keywords_pg(&self.pool, &movie_ids),
            fetch_cast_pg(&self.pool, &movie_ids),
        )?;

        // 3) Build result
        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let movie_id_str: String = row.try_get("movie_id").map_err(map_err)?;
            let title: String = row.try_get("title").map_err(map_err)?;
            let release_year: i64 = row.try_get("release_year").map_err(map_err)?;
            let director: Option<String> = row.try_get("director").map_err(map_err)?;
            let poster_path: Option<String> = row.try_get("poster_path").map_err(map_err)?;
            let rating: i64 = row.try_get("rating").map_err(map_err)?;
            let watched_at_str: String = row.try_get("watched_at").map_err(map_err)?;
            let user_id_str: String = row.try_get("user_id").map_err(map_err)?;
            let runtime_minutes: Option<i32> = row.try_get("runtime_minutes").map_err(map_err)?;
            let budget_usd: Option<i64> = row.try_get("budget_usd").map_err(map_err)?;
            let original_language: Option<String> =
                row.try_get("original_language").map_err(map_err)?;

            let genres = genres_map.get(&movie_id_str).cloned().unwrap_or_default();
            let keywords = keywords_map.get(&movie_id_str).cloned().unwrap_or_default();
            let cast = cast_map.get(&movie_id_str).cloned().unwrap_or_default();

            let cast_names: Vec<(String, u32)> = cast
                .iter()
                .map(|c| (c.name.clone(), c.billing_order))
                .collect();
            let cast_profile_paths: Vec<Option<String>> =
                cast.iter().map(|c| c.profile_path.clone()).collect();

            result.push(WrapUpMovieRow {
                movie_id: parse_uuid(&movie_id_str)?,
                title,
                release_year: release_year as u16,
                director,
                poster_path,
                rating: rating as u8,
                watched_at: parse_datetime(&watched_at_str)?,
                user_id: parse_uuid(&user_id_str)?,
                runtime_minutes: runtime_minutes.map(|v| v as u32),
                budget_usd,
                original_language,
                genres,
                keywords,
                cast_names,
                cast_profile_paths,
            });
        }

        Ok(result)
    }
}

#[derive(Clone)]
struct CastEntry {
    name: String,
    billing_order: u32,
    profile_path: Option<String>,
}

async fn fetch_genres_pg(
    pool: &PgPool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DomainError> {
    let rows = sqlx::query(
        "SELECT movie_id, name FROM movie_genres WHERE movie_id = ANY($1) ORDER BY name",
    )
    .bind(movie_ids)
    .fetch_all(pool)
    .await
    .map_err(map_err)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row.try_get("movie_id").map_err(map_err)?;
        let name: String = row.try_get("name").map_err(map_err)?;
        map.entry(mid).or_default().push(name);
    }
    Ok(map)
}

async fn fetch_keywords_pg(
    pool: &PgPool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DomainError> {
    let rows = sqlx::query(
        "SELECT movie_id, name FROM movie_keywords WHERE movie_id = ANY($1) ORDER BY name",
    )
    .bind(movie_ids)
    .fetch_all(pool)
    .await
    .map_err(map_err)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row.try_get("movie_id").map_err(map_err)?;
        let name: String = row.try_get("name").map_err(map_err)?;
        map.entry(mid).or_default().push(name);
    }
    Ok(map)
}

async fn fetch_cast_pg(
    pool: &PgPool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<CastEntry>>, DomainError> {
    let rows = sqlx::query(
        "SELECT movie_id, name, billing_order, profile_path \
         FROM movie_cast \
         WHERE movie_id = ANY($1) AND billing_order <= 3 \
         ORDER BY billing_order ASC",
    )
    .bind(movie_ids)
    .fetch_all(pool)
    .await
    .map_err(map_err)?;

    let mut map: HashMap<String, Vec<CastEntry>> = HashMap::new();
    for row in rows {
        let mid: String = row.try_get("movie_id").map_err(map_err)?;
        let name: String = row.try_get("name").map_err(map_err)?;
        let billing_order: i32 = row.try_get("billing_order").map_err(map_err)?;
        let profile_path: Option<String> = row.try_get("profile_path").map_err(map_err)?;
        map.entry(mid).or_default().push(CastEntry {
            name,
            billing_order: billing_order as u32,
            profile_path,
        });
    }
    Ok(map)
}
