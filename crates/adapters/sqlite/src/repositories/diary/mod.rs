use domain::{errors::DomainError, models::ReviewSortBy};
use sqlx::SqlitePool;

use rows::DiaryRow;

mod query;
mod rows;

pub struct SqliteDiaryRepository {
    pool: SqlitePool,
}

impl SqliteDiaryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn count_diary_entries(&self, movie_id: Option<&str>) -> Result<i64, DomainError> {
        match movie_id {
            None => sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews")
                .fetch_one(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error),
            Some(id) => {
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews WHERE movie_id = ?")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(crate::shared::map_sqlx_error)
            }
        }
    }

    async fn fetch_all_diary_rows(
        &self,
        sort: &ReviewSortBy,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let order_clause = match sort {
            ReviewSortBy::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            ReviewSortBy::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            ReviewSortBy::Ascending => "r.watched_at ASC",
            ReviewSortBy::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             ORDER BY {}
             LIMIT ? OFFSET ?",
            order_clause
        );
        sqlx::query_as::<_, DiaryRow>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)
    }

    async fn fetch_movie_diary_rows(
        &self,
        movie_id: &str,
        sort: &ReviewSortBy,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let order_clause = match sort {
            ReviewSortBy::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            ReviewSortBy::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            ReviewSortBy::Ascending => "r.watched_at ASC",
            ReviewSortBy::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = ?
             ORDER BY {}
             LIMIT ? OFFSET ?",
            order_clause
        );
        sqlx::query_as::<_, DiaryRow>(&sql)
            .bind(movie_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)
    }

    async fn count_user_diary_entries(
        &self,
        user_id: &str,
        search: Option<&str>,
        include_remote: bool,
    ) -> Result<i64, DomainError> {
        let has_search = search.map(|s| !s.is_empty()).unwrap_or(false);
        let remote_clause = if include_remote {
            ""
        } else {
            " AND r.remote_actor_url IS NULL"
        };
        let search_clause = if has_search {
            " AND m.title LIKE '%' || ? || '%'"
        } else {
            ""
        };
        let sql = format!(
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?{remote_clause}{search_clause}"
        );
        let mut q = sqlx::query_scalar::<_, i64>(&sql).bind(user_id);
        if has_search {
            q = q.bind(search.unwrap());
        }
        q.fetch_one(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)
    }

    async fn fetch_user_diary_rows(
        &self,
        user_id: &str,
        sort: &ReviewSortBy,
        search: Option<&str>,
        include_remote: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let has_search = search.map(|s| !s.is_empty()).unwrap_or(false);
        let remote_clause = if include_remote {
            ""
        } else {
            " AND r.remote_actor_url IS NULL"
        };
        let search_clause = if has_search {
            " AND m.title LIKE '%' || ? || '%'"
        } else {
            ""
        };
        let order_clause = match sort {
            ReviewSortBy::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            ReviewSortBy::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            ReviewSortBy::Ascending => "r.watched_at ASC",
            ReviewSortBy::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?{remote_clause}{search_clause}
             ORDER BY {order_clause}
             LIMIT ? OFFSET ?",
        );
        let mut q = sqlx::query_as::<_, DiaryRow>(&sql).bind(user_id);
        if has_search {
            q = q.bind(search.unwrap());
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)
    }
}
