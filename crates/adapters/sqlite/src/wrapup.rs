use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    models::wrapup::{DateRange, WrapUpRecord, WrapUpScope, WrapUpStatus},
    ports::{WrapUpMovieRow, WrapUpRepository, WrapUpStatsQuery},
    value_objects::WrapUpId,
};
use sqlx::{Row, SqlitePool};
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

fn parse_date(s: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid date '{s}': {e}")))
}

// ── WrapUpRepository ─────────────────────────────────────────────────────────

pub struct SqliteWrapUpRepository {
    pool: SqlitePool,
}

impl SqliteWrapUpRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrapUpRepository for SqliteWrapUpRepository {
    async fn create(&self, record: &WrapUpRecord) -> Result<(), DomainError> {
        let id = record.id.value().to_string();
        let user_id = record.user_id.map(|u| u.to_string());
        let status = status_to_str(&record.status);
        let start = record.start_date.format("%Y-%m-%d").to_string();
        let end = record.end_date.format("%Y-%m-%d").to_string();
        let created = record.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = record
            .completed_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        sqlx::query(
            "INSERT INTO wrap_up_records \
             (id, user_id, start_date, end_date, status, report_json, error_message, created_at, completed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&start)
        .bind(&end)
        .bind(status)
        .bind(&record.report_json)
        .bind(&record.error_message)
        .bind(&created)
        .bind(&completed)
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

        sqlx::query("UPDATE wrap_up_records SET status = ?, error_message = ? WHERE id = ?")
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
             SET status = 'ready', report_json = ?, completed_at = strftime('%Y-%m-%d %H:%M:%S', 'now') \
             WHERE id = ?",
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
                    created_at, completed_at \
             FROM wrap_up_records WHERE id = ?",
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
                    created_at, completed_at \
             FROM wrap_up_records WHERE user_id = ? ORDER BY created_at DESC",
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
                    created_at, completed_at \
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
        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        let row = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    created_at, completed_at \
             FROM wrap_up_records \
             WHERE ((? IS NULL AND user_id IS NULL) OR user_id = ?) \
               AND start_date = ? AND end_date = ? \
             LIMIT 1",
        )
        .bind(&uid)
        .bind(&uid)
        .bind(&start_str)
        .bind(&end_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.as_ref().map(row_to_record).transpose()
    }

    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM wrap_up_records WHERE id = ?")
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
        let before_str = before.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let result = sqlx::query(
            "DELETE FROM wrap_up_records WHERE status = 'failed' AND created_at < ?",
        )
        .bind(&before_str)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result.rows_affected())
    }
}

fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> Result<WrapUpRecord, DomainError> {
    let id_str: String = row.try_get("id").map_err(map_err)?;
    let user_id_str: Option<String> = row.try_get("user_id").map_err(map_err)?;
    let start_date_str: String = row.try_get("start_date").map_err(map_err)?;
    let end_date_str: String = row.try_get("end_date").map_err(map_err)?;
    let status_str: String = row.try_get("status").map_err(map_err)?;
    let report_json: Option<String> = row.try_get("report_json").map_err(map_err)?;
    let error_message: Option<String> = row.try_get("error_message").map_err(map_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_err)?;
    let completed_at_str: Option<String> = row.try_get("completed_at").map_err(map_err)?;

    let user_id = user_id_str.as_deref().map(parse_uuid).transpose()?;

    Ok(WrapUpRecord {
        id: WrapUpId::from_uuid(parse_uuid(&id_str)?),
        user_id,
        start_date: parse_date(&start_date_str)?,
        end_date: parse_date(&end_date_str)?,
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

pub struct SqliteWrapUpStatsQuery {
    pool: SqlitePool,
}

impl SqliteWrapUpStatsQuery {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrapUpStatsQuery for SqliteWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        scope: &WrapUpScope,
        range: &DateRange,
    ) -> Result<Vec<WrapUpMovieRow>, DomainError> {
        let start_str = range.start.format("%Y-%m-%d").to_string();
        let end_str = range.end.format("%Y-%m-%d").to_string();

        // 1) Main query
        let (scope_clause, scope_bind) = match scope {
            WrapUpScope::User(uid) => ("AND r.user_id = ?", Some(uid.to_string())),
            WrapUpScope::Global => ("", None),
        };

        let sql = format!(
            "SELECT r.movie_id, m.title, m.release_year, m.director, m.poster_path, \
                    r.rating, r.watched_at, r.user_id, \
                    p.runtime_minutes, p.budget_usd, p.original_language \
             FROM reviews r \
             INNER JOIN movies m ON m.id = r.movie_id \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             WHERE r.watched_at >= ? AND r.watched_at < ? {scope_clause} \
             ORDER BY r.watched_at ASC"
        );

        let mut q = sqlx::query(&sql).bind(&start_str).bind(&end_str);
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
            fetch_genres_sqlite(&self.pool, &movie_ids),
            fetch_keywords_sqlite(&self.pool, &movie_ids),
            fetch_cast_sqlite(&self.pool, &movie_ids),
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

fn in_placeholders(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push('?');
    }
    s
}

async fn fetch_genres_sqlite(
    pool: &SqlitePool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DomainError> {
    if movie_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT movie_id, name FROM movie_genres WHERE movie_id IN ({}) ORDER BY name",
        in_placeholders(movie_ids.len())
    );
    let mut q = sqlx::query(&sql);
    for id in movie_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await.map_err(map_err)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row.try_get("movie_id").map_err(map_err)?;
        let name: String = row.try_get("name").map_err(map_err)?;
        map.entry(mid).or_default().push(name);
    }
    Ok(map)
}

async fn fetch_keywords_sqlite(
    pool: &SqlitePool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DomainError> {
    if movie_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT movie_id, name FROM movie_keywords WHERE movie_id IN ({}) ORDER BY name",
        in_placeholders(movie_ids.len())
    );
    let mut q = sqlx::query(&sql);
    for id in movie_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await.map_err(map_err)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row.try_get("movie_id").map_err(map_err)?;
        let name: String = row.try_get("name").map_err(map_err)?;
        map.entry(mid).or_default().push(name);
    }
    Ok(map)
}

async fn fetch_cast_sqlite(
    pool: &SqlitePool,
    movie_ids: &[String],
) -> Result<HashMap<String, Vec<CastEntry>>, DomainError> {
    if movie_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT movie_id, name, billing_order, profile_path \
         FROM movie_cast \
         WHERE movie_id IN ({}) AND billing_order <= 3 \
         ORDER BY billing_order ASC",
        in_placeholders(movie_ids.len())
    );
    let mut q = sqlx::query(&sql);
    for id in movie_ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await.map_err(map_err)?;

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
