use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, FeedEntry, MovieStats, ReviewHistory, SortDirection,
        collections::{PageParams, Paginated},
    },
    ports::DiaryRepository,
    value_objects::{MovieId, UserId},
};
use futures::stream::BoxStream;
use sqlx::SqlitePool;

use crate::models::{DiaryRow, FeedRow, MovieRow, MovieStatsRow, ReviewRow};

pub struct SqliteDiaryRepository {
    pool: SqlitePool,
}

impl SqliteDiaryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }

    async fn count_diary_entries(&self, movie_id: Option<&str>) -> Result<i64, DomainError> {
        match movie_id {
            None => sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err),
            Some(id) => {
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews WHERE movie_id = ?")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Self::map_err)
            }
        }
    }

    async fn fetch_all_diary_rows(
        &self,
        sort: &SortDirection,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let order_clause = match sort {
            SortDirection::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            SortDirection::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            SortDirection::Ascending => "r.watched_at ASC",
            SortDirection::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
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
            .map_err(Self::map_err)
    }

    async fn fetch_movie_diary_rows(
        &self,
        movie_id: &str,
        sort: &SortDirection,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let order_clause = match sort {
            SortDirection::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            SortDirection::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            SortDirection::Ascending => "r.watched_at ASC",
            SortDirection::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
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
            .map_err(Self::map_err)
    }

    async fn count_user_diary_entries(
        &self,
        user_id: &str,
        search: Option<&str>,
    ) -> Result<i64, DomainError> {
        let has_search = search.map(|s| !s.is_empty()).unwrap_or(false);
        let sql = if has_search {
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ? AND m.title LIKE '%' || ? || '%'"
                .to_string()
        } else {
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?"
                .to_string()
        };
        let mut q = sqlx::query_scalar::<_, i64>(&sql).bind(user_id);
        if has_search {
            q = q.bind(search.unwrap());
        }
        q.fetch_one(&self.pool).await.map_err(Self::map_err)
    }

    async fn fetch_user_diary_rows(
        &self,
        user_id: &str,
        sort: &SortDirection,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        let has_search = search.map(|s| !s.is_empty()).unwrap_or(false);
        let search_clause = if has_search {
            " AND m.title LIKE '%' || ? || '%'"
        } else {
            ""
        };
        let order_clause = match sort {
            SortDirection::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            SortDirection::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            SortDirection::Ascending => "r.watched_at ASC",
            SortDirection::Descending => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?{}
             ORDER BY {}
             LIMIT ? OFFSET ?",
            search_clause, order_clause
        );
        let mut q = sqlx::query_as::<_, DiaryRow>(&sql).bind(user_id);
        if has_search {
            q = q.bind(search.unwrap());
        }
        q.bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)
    }
}

#[async_trait]
impl DiaryRepository for SqliteDiaryRepository {
    async fn query_diary(
        &self,
        filter: &DiaryFilter,
    ) -> Result<Paginated<DiaryEntry>, DomainError> {
        let limit = filter.page.limit as i64;
        let offset = filter.page.offset as i64;

        let (total, rows) = match (&filter.movie_id, &filter.user_id) {
            (None, None) => tokio::try_join!(
                self.count_diary_entries(None),
                self.fetch_all_diary_rows(&filter.sort_by, limit, offset)
            )?,
            (Some(id), None) => {
                let id_str = id.value().to_string();
                tokio::try_join!(
                    self.count_diary_entries(Some(id_str.as_str())),
                    self.fetch_movie_diary_rows(&id_str, &filter.sort_by, limit, offset)
                )?
            }
            (None, Some(uid)) => {
                let uid_str = uid.value().to_string();
                let search = filter.search.as_deref();
                tokio::try_join!(
                    self.count_user_diary_entries(&uid_str, search),
                    self.fetch_user_diary_rows(&uid_str, &filter.sort_by, search, limit, offset)
                )?
            }
            (Some(_), Some(_)) => {
                return Err(DomainError::ValidationError(
                    "Combined movie_id + user_id filter not supported".into(),
                ));
            }
        };

        let items = rows
            .into_iter()
            .map(DiaryRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: filter.page.limit,
            offset: filter.page.offset,
        })
    }

    async fn query_activity_feed(
        &self,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        self.query_activity_feed_filtered(page, &domain::ports::FeedSortBy::Date, None, None)
            .await
    }

    async fn query_activity_feed_filtered(
        &self,
        page: &PageParams,
        sort_by: &domain::ports::FeedSortBy,
        search: Option<&str>,
        following: Option<&domain::ports::FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        use domain::ports::FeedSortBy;

        let limit = page.limit as i64;
        let offset = page.offset as i64;
        let has_search = search.map(|s| !s.is_empty()).unwrap_or(false);

        let mut where_parts = vec!["1=1".to_string()];

        if has_search {
            where_parts.push("m.title LIKE '%' || ? || '%'".to_string());
        }

        if let Some(f) = following {
            let local_in = if f.local_user_ids.is_empty() {
                "SELECT NULL WHERE 0".to_string()
            } else {
                f.local_user_ids
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let remote_in = if f.remote_actor_urls.is_empty() {
                "SELECT NULL WHERE 0".to_string()
            } else {
                f.remote_actor_urls
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",")
            };
            where_parts.push(format!(
                "(r.user_id IN ({}) OR r.remote_actor_url IN ({}))",
                local_in, remote_in
            ));
        }

        let order_clause = match sort_by {
            FeedSortBy::Date => "r.watched_at DESC",
            FeedSortBy::DateAsc => "r.watched_at ASC",
            FeedSortBy::Rating => "r.rating DESC, r.watched_at DESC",
            FeedSortBy::RatingAsc => "r.rating ASC, r.watched_at ASC",
        };

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE {}",
            where_clause
        );

        let select_sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    r.watched_at, r.created_at, r.remote_actor_url,
                    COALESCE(u.email, r.remote_actor_url) AS user_email
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             LEFT JOIN users u ON u.id = r.user_id
             WHERE {}
             ORDER BY {}
             LIMIT ? OFFSET ?",
            where_clause, order_clause
        );

        macro_rules! bind_filter_params {
            ($q:expr) => {{
                let mut q = $q;
                if has_search {
                    q = q.bind(search.unwrap());
                }
                if let Some(f) = following {
                    for uid in &f.local_user_ids {
                        q = q.bind(uid.to_string());
                    }
                    for url in &f.remote_actor_urls {
                        q = q.bind(url.as_str());
                    }
                }
                q
            }};
        }

        let count_q = bind_filter_params!(sqlx::query_scalar::<_, i64>(&count_sql));
        let total = count_q.fetch_one(&self.pool).await.map_err(Self::map_err)?;

        let rows_q = bind_filter_params!(sqlx::query_as::<_, FeedRow>(&select_sql));
        let rows = rows_q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?;

        let items = rows
            .into_iter()
            .map(FeedRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        let id_str = movie_id.value().to_string();

        let movie = sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", id_str)))?
        .into_domain()?;

        let viewings = sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url
             FROM reviews WHERE movie_id = ? ORDER BY watched_at ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(ReviewRow::into_domain)
        .collect::<Result<Vec<_>, _>>()?;

        Ok(ReviewHistory::new(movie, viewings))
    }

    async fn get_user_history(&self, user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?
             ORDER BY r.watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    fn stream_user_history(
        &self,
        user_id: UserId,
    ) -> BoxStream<'static, Result<DiaryEntry, DomainError>> {
        let pool = self.pool.clone();
        let uid = user_id.value().to_string();
        Box::pin(async_stream::stream! {
            let mut rows = sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ?
                 ORDER BY r.watched_at DESC",
            )
            .bind(&uid)
            .fetch(&pool);
            while let Some(row) = futures::StreamExt::next(&mut rows).await {
                yield match row {
                    Ok(r) => r.into_domain(),
                    Err(e) => Err(Self::map_err(e)),
                };
            }
        })
    }

    async fn get_movie_stats(&self, movie_id: &MovieId) -> Result<MovieStats, DomainError> {
        let id_str = movie_id.value().to_string();
        sqlx::query_as::<_, MovieStatsRow>(
            "SELECT
                COUNT(*) AS total_count,
                AVG(CAST(rating AS REAL)) AS avg_rating,
                COUNT(CASE WHEN remote_actor_url IS NOT NULL THEN 1 END) AS federated_count,
                COUNT(CASE WHEN rating = 1 THEN 1 END) AS rating_1,
                COUNT(CASE WHEN rating = 2 THEN 1 END) AS rating_2,
                COUNT(CASE WHEN rating = 3 THEN 1 END) AS rating_3,
                COUNT(CASE WHEN rating = 4 THEN 1 END) AS rating_4,
                COUNT(CASE WHEN rating = 5 THEN 1 END) AS rating_5
             FROM reviews WHERE movie_id = ?",
        )
        .bind(id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)
        .map(MovieStatsRow::into_domain)
    }

    async fn get_movie_social_feed(
        &self,
        movie_id: &MovieId,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        let id_str = movie_id.value().to_string();
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        let total: i64 =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews WHERE movie_id = ?")
                .bind(&id_str)
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err)?;

        let rows = sqlx::query_as::<_, FeedRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    r.watched_at, r.created_at, r.remote_actor_url,
                    CASE WHEN r.remote_actor_url IS NOT NULL THEN r.remote_actor_url
                         WHEN u.email IS NOT NULL THEN u.email
                         ELSE r.user_id END AS user_email
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             LEFT JOIN users u ON u.id = r.user_id
             WHERE r.movie_id = ?
             ORDER BY r.watched_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(&id_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let items = rows
            .into_iter()
            .map(FeedRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE remote_actor_url IS NULL")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err)?;
        Ok(count as u64)
    }
}

#[cfg(test)]
#[path = "tests/diary.rs"]
mod tests;
