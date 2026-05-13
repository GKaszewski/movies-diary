use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, DirectorStat, FeedEntry, MonthlyRating, Movie, MovieStats, Review,
        ReviewHistory, ReviewSource, SortDirection, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::{DiaryRepository, MovieRepository, ReviewRepository, StatsRepository},
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear, ReviewId, UserId},
};
use sqlx::SqlitePool;

mod image_ref;
mod import_profile;
mod import_session;
mod migrations;
mod models;
mod persons;
mod profile;
mod profile_fields;
mod users;
mod watchlist;

use models::{
    DiaryRow, DirectorCountRow, FeedRow, MonthlyRatingRow, MovieRow, MovieStatsRow,
    MovieSummaryRow, ReviewRow, UserTotalsRow, datetime_to_str,
};

pub use image_ref::{SqliteImageRefAdapter, create_image_ref};
pub use import_profile::SqliteImportProfileRepository;
pub use import_session::SqliteImportSessionRepository;
pub use persons::{SqlitePersonAdapter, create_person_adapter};
pub use profile::SqliteMovieProfileRepository;
pub use profile_fields::SqliteProfileFieldsRepository;
pub use users::SqliteUserRepository;
pub use watchlist::SqliteWatchlistRepository;

pub fn create_profile_fields_repo(
    pool: sqlx::SqlitePool,
) -> std::sync::Arc<dyn domain::ports::UserProfileFieldsRepository> {
    std::sync::Arc::new(SqliteProfileFieldsRepository::new(pool))
}

fn format_year_month(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 {
        return ym.to_string();
    }
    let year = parts[0].get(2..).unwrap_or(parts[0]);
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    format!("{} '{}", month, year)
}

pub struct SqliteMovieRepository {
    pool: SqlitePool,
}

impl SqliteMovieRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), DomainError> {
        migrations::run(&self.pool).await
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }

    async fn count_diary_entries(&self, movie_id: Option<&str>) -> Result<i64, DomainError> {
        match movie_id {
            None => sqlx::query_scalar!("SELECT COUNT(*) FROM reviews")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err),
            Some(id) => sqlx::query_scalar!("SELECT COUNT(*) FROM reviews WHERE movie_id = ?", id)
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err),
        }
    }

    async fn fetch_all_diary_rows(
        &self,
        sort: &SortDirection,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        match sort {
            // ByRatingDesc/ByRatingAsc only apply to user-scoped queries; fall back to date sort here
            SortDirection::Descending | SortDirection::ByRatingDesc | SortDirection::ByRatingAsc => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 ORDER BY r.watched_at DESC
                 LIMIT ? OFFSET ?",
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),

            SortDirection::Ascending => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 ORDER BY r.watched_at ASC
                 LIMIT ? OFFSET ?",
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),
        }
    }

    async fn fetch_movie_diary_rows(
        &self,
        movie_id: &str,
        sort: &SortDirection,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        match sort {
            // ByRatingDesc/ByRatingAsc only apply to user-scoped queries; fall back to date sort here
            SortDirection::Descending | SortDirection::ByRatingDesc | SortDirection::ByRatingAsc => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.movie_id = ?
                 ORDER BY r.watched_at DESC
                 LIMIT ? OFFSET ?",
                movie_id,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),

            SortDirection::Ascending => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.movie_id = ?
                 ORDER BY r.watched_at ASC
                 LIMIT ? OFFSET ?",
                movie_id,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),
        }
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

    async fn fetch_user_totals(&self, user_id: &str) -> Result<UserTotalsRow, DomainError> {
        sqlx::query_as!(
            UserTotalsRow,
            r#"SELECT COUNT(DISTINCT movie_id) AS "total!: i64",
                      AVG(CAST(rating AS REAL)) AS avg_rating
               FROM reviews WHERE user_id = ?"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn fetch_user_favorite_director(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        let row = sqlx::query_scalar!(
            "SELECT m.director
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ? AND m.director IS NOT NULL
             GROUP BY m.director
             ORDER BY COUNT(*) DESC
             LIMIT 1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(row.flatten())
    }

    async fn fetch_user_most_active_month(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        let result: Option<Option<String>> = sqlx::query_scalar!(
            "SELECT strftime('%Y-%m', watched_at) AS month
             FROM reviews
             WHERE user_id = ?
             GROUP BY month
             ORDER BY COUNT(*) DESC
             LIMIT 1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(result.flatten())
    }
}

#[async_trait]
impl MovieRepository for SqliteMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let id = external_metadata_id.value();
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let title = title.value();
        let year = year.value() as i64;
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE title = ? AND release_year = ?",
            title,
            year
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(MovieRow::into_domain)
        .collect()
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        sqlx::query!(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = excluded.external_metadata_id,
                 title                = excluded.title,
                 release_year         = excluded.release_year,
                 director             = excluded.director,
                 poster_path          = excluded.poster_path",
            id,
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query!("DELETE FROM movies WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn list_movies(
        &self,
        page: &domain::models::collections::PageParams,
        filter: &domain::models::MovieFilter,
    ) -> Result<domain::models::collections::Paginated<domain::models::MovieSummary>, DomainError>
    {
        use sqlx::Row;
        let limit = page.limit as i64;
        let offset = page.offset as i64;
        let pattern = filter
            .search
            .as_deref()
            .map(|s| format!("%{}%", s.to_lowercase()));
        let genre = filter.genre.as_deref();
        let language = filter.language.as_deref();

        let rows: Vec<MovieSummaryRow> = sqlx::query_as(
            "SELECT \
               m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path, \
               p.overview, p.runtime_minutes, p.original_language, p.collection_name, \
               GROUP_CONCAT(g.name) AS genres \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             LEFT JOIN movie_genres g ON g.movie_id = m.id \
             WHERE (? IS NULL OR LOWER(m.title) LIKE ?) \
               AND (? IS NULL OR p.original_language = ?) \
               AND (? IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER(?))) \
             GROUP BY m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path, \
                      p.overview, p.runtime_minutes, p.original_language, p.collection_name \
             ORDER BY m.title ASC \
             LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(language)
        .bind(language)
        .bind(genre)
        .bind(genre)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let total: i64 = sqlx::query(
            "SELECT COUNT(DISTINCT m.id) \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             WHERE (? IS NULL OR LOWER(m.title) LIKE ?) \
               AND (? IS NULL OR p.original_language = ?) \
               AND (? IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER(?)))",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(language)
        .bind(language)
        .bind(genre)
        .bind(genre)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?
        .try_get(0)
        .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|r| r.into_domain())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(domain::models::collections::Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }
}

#[async_trait]
impl ReviewRepository for SqliteMovieRepository {
    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError> {
        let id = review.id().value().to_string();
        let movie_id = review.movie_id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());
        let remote_actor_url = match review.source() {
            ReviewSource::Local => None,
            ReviewSource::Remote { actor_url } => Some(actor_url.clone()),
        };

        sqlx::query!(
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            remote_actor_url
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(DomainEvent::ReviewLogged {
            review_id: review.id().clone(),
            movie_id: review.movie_id().clone(),
            user_id: review.user_id().clone(),
            rating: review.rating().clone(),
            watched_at: *review.watched_at(),
        })
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        let id = review_id.value().to_string();
        sqlx::query_as!(
            ReviewRow,
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url
             FROM reviews WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(ReviewRow::into_domain)
        .transpose()
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        let id = review_id.value().to_string();
        sqlx::query!("DELETE FROM reviews WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn get_all_reviews_for_user(
        &self,
        _user_id: &UserId,
    ) -> Result<Vec<Review>, DomainError> {
        todo!()
    }
}

#[async_trait]
impl DiaryRepository for SqliteMovieRepository {
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

        let movie = sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", id_str)))?
        .into_domain()?;

        let viewings = sqlx::query_as!(
            ReviewRow,
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url
             FROM reviews WHERE movie_id = ? ORDER BY watched_at ASC",
            id_str
        )
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
        let rows = sqlx::query_as!(
            DiaryRow,
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?
             ORDER BY r.watched_at DESC",
            uid
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter().map(DiaryRow::into_domain).collect()
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

        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM reviews WHERE movie_id = ?", id_str)
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

#[async_trait]
impl StatsRepository for SqliteMovieRepository {
    async fn get_user_stats(&self, user_id: &UserId) -> Result<UserStats, DomainError> {
        let uid = user_id.value().to_string();

        let (totals, fav_director, most_active) = tokio::try_join!(
            self.fetch_user_totals(&uid),
            self.fetch_user_favorite_director(&uid),
            self.fetch_user_most_active_month(&uid)
        )?;

        let most_active_month = most_active.map(|ym| format_year_month(&ym));

        Ok(UserStats {
            total_movies: totals.total,
            avg_rating: totals.avg_rating,
            favorite_director: fav_director,
            most_active_month,
        })
    }

    async fn get_user_trends(&self, user_id: &UserId) -> Result<UserTrends, DomainError> {
        let uid = user_id.value().to_string();

        let (rating_rows, director_rows) = tokio::try_join!(
            sqlx::query_as!(
                MonthlyRatingRow,
                r#"SELECT strftime('%Y-%m', watched_at) AS "month!",
                          AVG(CAST(rating AS REAL)) AS "avg_rating!: f64",
                          COUNT(*) AS "count!: i64"
                   FROM reviews
                   WHERE user_id = ? AND watched_at >= datetime('now', '-12 months')
                   GROUP BY "month!"
                   ORDER BY "month!" ASC"#,
                uid
            )
            .fetch_all(&self.pool),
            sqlx::query_as!(
                DirectorCountRow,
                r#"SELECT m.director AS "director!",
                          COUNT(*) AS "count!: i64"
                   FROM reviews r
                   INNER JOIN movies m ON m.id = r.movie_id
                   WHERE r.user_id = ? AND m.director IS NOT NULL
                   GROUP BY m.director
                   ORDER BY COUNT(*) DESC
                   LIMIT 5"#,
                uid
            )
            .fetch_all(&self.pool)
        )
        .map_err(Self::map_err)?;

        let max_director_count = director_rows.iter().map(|d| d.count).max().unwrap_or(1);

        let monthly_ratings = rating_rows
            .into_iter()
            .map(|r| MonthlyRating {
                month_label: format_year_month(&r.month),
                year_month: r.month,
                avg_rating: r.avg_rating,
                count: r.count,
            })
            .collect();

        let top_directors = director_rows
            .into_iter()
            .map(|d| DirectorStat {
                director: d.director,
                count: d.count,
            })
            .collect();

        Ok(UserTrends {
            monthly_ratings,
            top_directors,
            max_director_count,
        })
    }
}

pub async fn wire(
    database_url: &str,
) -> anyhow::Result<(
    sqlx::SqlitePool,
    std::sync::Arc<dyn domain::ports::MovieRepository>,
    std::sync::Arc<dyn domain::ports::ReviewRepository>,
    std::sync::Arc<dyn domain::ports::DiaryRepository>,
    std::sync::Arc<dyn domain::ports::StatsRepository>,
    std::sync::Arc<dyn domain::ports::UserRepository>,
    std::sync::Arc<dyn domain::ports::ImportSessionRepository>,
    std::sync::Arc<dyn domain::ports::ImportProfileRepository>,
    std::sync::Arc<dyn domain::ports::MovieProfileRepository>,
    std::sync::Arc<dyn domain::ports::WatchlistRepository>,
)> {
    use anyhow::Context;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let opts = SqliteConnectOptions::from_str(database_url)
        .context("Invalid DATABASE_URL")?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::SqlitePool::connect_with(opts)
        .await
        .context("Failed to connect to SQLite database")?;

    let repo = std::sync::Arc::new(SqliteMovieRepository::new(pool.clone()));
    repo.migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Database migration failed")?;

    let import_session_repo = std::sync::Arc::new(SqliteImportSessionRepository::new(pool.clone()));
    let import_profile_repo = std::sync::Arc::new(SqliteImportProfileRepository::new(pool.clone()));
    let movie_profile_repo = std::sync::Arc::new(SqliteMovieProfileRepository::new(pool.clone()));
    let watchlist_repo = std::sync::Arc::new(SqliteWatchlistRepository::new(pool.clone()));

    Ok((
        pool.clone(),
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::new(SqliteUserRepository::new(pool)) as _,
        import_session_repo as _,
        import_profile_repo as _,
        movie_profile_repo as _,
        watchlist_repo as _,
    ))
}

#[cfg(test)]
mod feed_filter_tests {
    use super::*;
    use domain::{
        models::collections::PageParams,
        ports::{DiaryRepository, FeedSortBy, FollowingFilter},
    };
    use sqlx::SqlitePool;

    async fn setup(pool: &SqlitePool) {
        sqlx::migrate!("./migrations").run(pool).await.unwrap();

        // carol is a remote actor; we still need a non-null user_id for the schema,
        // so we create a local "ghost" user and link the remote review via remote_actor_url.
        sqlx::query(
            "INSERT INTO users (id, email, username, password_hash, created_at) VALUES
             ('11111111-1111-1111-1111-111111111111', 'alice@example.com', 'alice', 'hash', '2024-01-01 00:00:00'),
             ('22222222-2222-2222-2222-222222222222', 'bob@example.com', 'bob', 'hash', '2024-01-01 00:00:00'),
             ('33333333-3333-3333-3333-333333333333', 'carol@remote.social', 'carol', 'hash', '2024-01-01 00:00:00')",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO movies (id, title, release_year) VALUES
             ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Inception', 2010),
             ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'Interstellar', 2014),
             ('cccccccc-cccc-cccc-cccc-cccccccccccc', 'Dune', 2021)",
        )
        .execute(pool)
        .await
        .unwrap();

        // carol's review: local user_id=33333333, remote_actor_url set → remote review
        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at, remote_actor_url) VALUES
             ('a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '11111111-1111-1111-1111-111111111111', 5, '2024-01-01 00:00:00', '2024-01-01 00:00:00', NULL),
             ('b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', '22222222-2222-2222-2222-222222222222', 3, '2024-01-02 00:00:00', '2024-01-02 00:00:00', NULL),
             ('c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3', 'cccccccc-cccc-cccc-cccc-cccccccccccc', '33333333-3333-3333-3333-333333333333', 4, '2024-01-03 00:00:00', '2024-01-03 00:00:00', 'https://remote.social/users/carol')",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_sort_by_rating_descending() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let page = PageParams::new(Some(10), Some(0)).unwrap();
        let result = repo
            .query_activity_feed_filtered(&page, &FeedSortBy::Rating, None, None)
            .await
            .unwrap();

        let ratings: Vec<u8> = result
            .items
            .iter()
            .map(|e| e.review().rating().value())
            .collect();
        assert_eq!(ratings, vec![5, 4, 3]);
    }

    #[tokio::test]
    async fn test_search_by_title() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let page = PageParams::new(Some(10), Some(0)).unwrap();
        let result = repo
            .query_activity_feed_filtered(&page, &FeedSortBy::Date, Some("Dune"), None)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].movie().title().value(), "Dune");
    }

    #[tokio::test]
    async fn test_following_filter() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let filter = FollowingFilter {
            local_user_ids: vec![
                uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            ],
            remote_actor_urls: vec!["https://remote.social/users/carol".to_string()],
        };
        let page = PageParams::new(Some(10), Some(0)).unwrap();
        let result = repo
            .query_activity_feed_filtered(&page, &FeedSortBy::Date, None, Some(&filter))
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2); // alice + carol, NOT bob
        let titles: Vec<String> = result
            .items
            .iter()
            .map(|e| e.movie().title().value().to_string())
            .collect();
        assert!(titles.contains(&"Inception".to_string()));
        assert!(titles.contains(&"Dune".to_string()));
    }

    #[tokio::test]
    async fn test_get_movie_stats_local() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        // Inception: 1 local review, rating=5, no federated
        let movie_id = domain::value_objects::MovieId::from_uuid(
            uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        );
        let stats = repo.get_movie_stats(&movie_id).await.unwrap();

        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.federated_count, 0);
        assert!((stats.avg_rating.unwrap() - 5.0).abs() < 0.001);
        assert_eq!(stats.rating_histogram[4], 1); // 5★ bucket
        assert_eq!(stats.rating_histogram[0], 0); // 1★ bucket
    }

    #[tokio::test]
    async fn test_get_movie_social_feed_returns_reviews_for_movie() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let movie_id = domain::value_objects::MovieId::from_uuid(
            uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        );
        let page = PageParams::new(Some(10), Some(0)).unwrap();
        let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].movie().title().value(), "Inception");
        assert_eq!(result.items[0].review().rating().value(), 5);
        assert_eq!(result.items[0].user_display_name(), "alice");
        assert!(!result.items[0].review().is_remote());
    }

    #[tokio::test]
    async fn test_get_movie_social_feed_federated_review() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let movie_id = domain::value_objects::MovieId::from_uuid(
            uuid::Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
        );
        let page = PageParams::new(Some(10), Some(0)).unwrap();
        let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.items.len(), 1);
        assert!(result.items[0].review().is_remote());
        assert_eq!(
            result.items[0].user_email(),
            "https://remote.social/users/carol"
        );
    }

    #[tokio::test]
    async fn test_get_movie_social_feed_pagination() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        let movie_id = domain::value_objects::MovieId::from_uuid(
            uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        );
        // offset beyond results: total_count still correct, items empty
        let page = PageParams::new(Some(10), Some(5)).unwrap();
        let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.items.len(), 0);
    }

    #[tokio::test]
    async fn test_get_movie_stats_federated() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup(&pool).await;
        let repo = SqliteMovieRepository::new(pool);

        // Dune: 1 federated review, rating=4
        let movie_id = domain::value_objects::MovieId::from_uuid(
            uuid::Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
        );
        let stats = repo.get_movie_stats(&movie_id).await.unwrap();

        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.federated_count, 1);
        assert_eq!(stats.rating_histogram[3], 1); // 4★ bucket
        assert_eq!(stats.rating_histogram[4], 0); // 5★ bucket
    }
}

#[cfg(test)]
mod diary_count_tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn count_local_posts_excludes_remote_reviews() {
        use domain::ports::DiaryRepository;
        let pool = test_pool().await;
        let repo = SqliteMovieRepository::new(pool.clone());

        let user_id = uuid::Uuid::new_v4().to_string();
        let movie_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, password_hash, created_at, username) VALUES (?, ?, ?, ?, ?)")
            .bind(&user_id).bind("a@b.com").bind("hash").bind("2024-01-01 00:00:00").bind("alice")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO movies (id, title, release_year) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind("Test Movie")
            .bind(2024i32)
            .execute(&pool)
            .await
            .unwrap();

        // Local review (remote_actor_url IS NULL)
        let r1 = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&r1).bind(&movie_id).bind(&user_id).bind(4i32)
            .bind("2024-01-01 00:00:00").bind("2024-01-01 00:00:00")
            .execute(&pool).await.unwrap();

        // Remote review (remote_actor_url IS NOT NULL)
        let r2 = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at, remote_actor_url) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&r2).bind(&movie_id).bind(&user_id).bind(3i32)
            .bind("2024-01-01 00:00:00").bind("2024-01-01 00:00:00").bind("https://remote/user")
            .execute(&pool).await.unwrap();

        let count = repo.count_local_posts().await.unwrap();
        assert_eq!(count, 1);
    }
}
