use std::collections::HashMap;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::wrapup::{DateRange, WrapUpMovieRow, WrapUpScope},
    ports::WrapUpStatsQuery,
};
use sqlx::{Row, SqlitePool};

use crate::shared::{parse_datetime, parse_uuid};

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
        let start_str = range.start().format("%Y-%m-%d").to_string();
        let end_str = range.end().format("%Y-%m-%d").to_string();

        // 1) Main query
        let (scope_clause, scope_bind) = match scope {
            WrapUpScope::User(uid) => ("AND r.user_id = ?", Some(uid.to_string())),
            WrapUpScope::Global => ("", None),
        };

        let sql = format!(
            "SELECT r.movie_id, m.title, m.release_year, m.director, m.poster_path, \
                    r.rating, r.watched_at, r.user_id, r.watch_medium, \
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

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // Collect unique movie IDs
        let mut movie_ids: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in &rows {
            let mid: String = row
                .try_get("movie_id")
                .map_err(crate::shared::map_sqlx_error)?;
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
            let movie_id_str: String = row
                .try_get("movie_id")
                .map_err(crate::shared::map_sqlx_error)?;
            let title: String = row
                .try_get("title")
                .map_err(crate::shared::map_sqlx_error)?;
            let release_year: i64 = row
                .try_get("release_year")
                .map_err(crate::shared::map_sqlx_error)?;
            let director: Option<String> = row
                .try_get("director")
                .map_err(crate::shared::map_sqlx_error)?;
            let poster_path: Option<String> = row
                .try_get("poster_path")
                .map_err(crate::shared::map_sqlx_error)?;
            let rating: i64 = row
                .try_get("rating")
                .map_err(crate::shared::map_sqlx_error)?;
            let watched_at_str: String = row
                .try_get("watched_at")
                .map_err(crate::shared::map_sqlx_error)?;
            let user_id_str: String = row
                .try_get("user_id")
                .map_err(crate::shared::map_sqlx_error)?;
            let runtime_minutes: Option<i32> = row
                .try_get("runtime_minutes")
                .map_err(crate::shared::map_sqlx_error)?;
            let budget_usd: Option<i64> = row
                .try_get("budget_usd")
                .map_err(crate::shared::map_sqlx_error)?;
            let original_language: Option<String> = row
                .try_get("original_language")
                .map_err(crate::shared::map_sqlx_error)?;
            let watch_medium: Option<String> = row
                .try_get("watch_medium")
                .map_err(crate::shared::map_sqlx_error)?;

            let genres = genres_map.get(&movie_id_str).cloned().unwrap_or_default();
            let keywords = keywords_map.get(&movie_id_str).cloned().unwrap_or_default();
            let cast = cast_map.get(&movie_id_str).cloned().unwrap_or_default();

            let cast_names: Vec<(String, u32, i64)> = cast
                .iter()
                .map(|c| (c.name.clone(), c.billing_order, c.tmdb_person_id))
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
                watch_medium,
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
    tmdb_person_id: i64,
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

/// Shared by WrapUp and Insights: movie-id → genre names.
pub(crate) async fn fetch_genres_sqlite(
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
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row
            .try_get("movie_id")
            .map_err(crate::shared::map_sqlx_error)?;
        let name: String = row.try_get("name").map_err(crate::shared::map_sqlx_error)?;
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
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let mid: String = row
            .try_get("movie_id")
            .map_err(crate::shared::map_sqlx_error)?;
        let name: String = row.try_get("name").map_err(crate::shared::map_sqlx_error)?;
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
        "SELECT movie_id, name, billing_order, tmdb_person_id, profile_path \
         FROM movie_cast \
         WHERE movie_id IN ({}) AND billing_order <= 3 \
         ORDER BY billing_order ASC",
        in_placeholders(movie_ids.len())
    );
    let mut q = sqlx::query(&sql);
    for id in movie_ids {
        q = q.bind(id);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

    let mut map: HashMap<String, Vec<CastEntry>> = HashMap::new();
    for row in rows {
        let mid: String = row
            .try_get("movie_id")
            .map_err(crate::shared::map_sqlx_error)?;
        let name: String = row.try_get("name").map_err(crate::shared::map_sqlx_error)?;
        let billing_order: i32 = row
            .try_get("billing_order")
            .map_err(crate::shared::map_sqlx_error)?;
        let tmdb_person_id: i64 = row
            .try_get("tmdb_person_id")
            .map_err(crate::shared::map_sqlx_error)?;
        let profile_path: Option<String> = row
            .try_get("profile_path")
            .map_err(crate::shared::map_sqlx_error)?;
        map.entry(mid).or_default().push(CastEntry {
            name,
            billing_order: billing_order as u32,
            tmdb_person_id,
            profile_path,
        });
    }
    Ok(map)
}
