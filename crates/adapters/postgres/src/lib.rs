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
use sqlx::PgPool;

mod import_profile;
mod import_session;
mod models;
mod profile;
mod users;

use models::{
    DiaryRow, DirectorCountRow, FeedRow, MonthlyRatingRow, MovieRow, MovieStatsRow, ReviewRow,
    UserTotalsRow, datetime_to_str,
};

pub use import_profile::PostgresImportProfileRepository;
pub use import_session::PostgresImportSessionRepository;
pub use profile::PostgresMovieProfileRepository;
pub use users::PostgresUserRepository;

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

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), DomainError> {
        sqlx::migrate!("./migrations")
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Migration failed: {}", e)))
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
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews WHERE movie_id = $1")
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
        let order = match sort {
            SortDirection::Ascending => "r.watched_at ASC",
            _ => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             ORDER BY {}
             LIMIT $1 OFFSET $2",
            order
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
        let order = match sort {
            SortDirection::Ascending => "r.watched_at ASC",
            _ => "r.watched_at DESC",
        };
        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = $1
             ORDER BY {}
             LIMIT $2 OFFSET $3",
            order
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
             WHERE r.user_id = $1 AND m.title ILIKE '%' || $2 || '%'"
                .to_string()
        } else {
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1"
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
        let order_clause = match sort {
            SortDirection::ByRatingDesc => "r.rating DESC, r.watched_at DESC",
            SortDirection::ByRatingAsc => "r.rating ASC, r.watched_at ASC",
            SortDirection::Ascending => "r.watched_at ASC",
            SortDirection::Descending => "r.watched_at DESC",
        };

        // Build param counter: user_id=$1, optional search=$2, limit=$N-1, offset=$N
        let mut p: i32 = 1; // $1 is user_id
        let search_clause = if has_search {
            p += 1;
            format!(" AND m.title ILIKE '%' || ${} || '%'", p)
        } else {
            String::new()
        };
        p += 1;
        let limit_param = format!("${}", p);
        p += 1;
        let offset_param = format!("${}", p);

        let sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1{}
             ORDER BY {}
             LIMIT {} OFFSET {}",
            search_clause, order_clause, limit_param, offset_param
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
        sqlx::query_as::<_, UserTotalsRow>(
            r#"SELECT COUNT(DISTINCT movie_id) AS total,
                      AVG(rating::float) AS avg_rating
               FROM reviews WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn fetch_user_favorite_director(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        sqlx::query_scalar::<_, String>(
            "SELECT m.director
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1 AND m.director IS NOT NULL
             GROUP BY m.director
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn fetch_user_most_active_month(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        sqlx::query_scalar::<_, String>(
            "SELECT to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') AS month
             FROM reviews
             WHERE user_id = $1
             GROUP BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM')
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)
    }
}

#[async_trait]
impl MovieRepository for PostgresRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let id = external_metadata_id.value();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::to_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::to_domain)
        .transpose()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let title = title.value();
        let year = year.value() as i64;
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE title = $1 AND release_year = $2",
        )
        .bind(title)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(MovieRow::to_domain)
        .collect()
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = excluded.external_metadata_id,
                 title                = excluded.title,
                 release_year         = excluded.release_year,
                 director             = excluded.director,
                 poster_path          = excluded.poster_path",
        )
        .bind(&id)
        .bind(&external_metadata_id)
        .bind(title)
        .bind(release_year)
        .bind(director)
        .bind(&poster_path)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query("DELETE FROM movies WHERE id = $1")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn list_movies(
        &self,
        page: &domain::models::collections::PageParams,
        search: Option<&str>,
    ) -> Result<domain::models::collections::Paginated<domain::models::Movie>, DomainError> {
        use sqlx::Row;
        let limit = page.limit as i64;
        let offset = page.offset as i64;
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));

        let rows: Vec<models::MovieRow> = sqlx::query_as(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path \
             FROM movies \
             WHERE ($1::text IS NULL OR LOWER(title) LIKE $1) \
             ORDER BY title ASC \
             LIMIT $2 OFFSET $3",
        )
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let total: i64 = sqlx::query(
            "SELECT COUNT(*) FROM movies WHERE ($1::text IS NULL OR LOWER(title) LIKE $1)",
        )
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?
        .try_get(0)
        .unwrap_or(0);

        let items = rows.into_iter()
            .map(|r| r.to_domain())
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
impl ReviewRepository for PostgresRepository {
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

        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url)
             VALUES ($1, $2, $3, $4, $5, $6::timestamptz, $7::timestamptz, $8)",
        )
        .bind(&id)
        .bind(&movie_id)
        .bind(&user_id)
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&created_at)
        .bind(&remote_actor_url)
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
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment,
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    remote_actor_url
             FROM reviews WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(ReviewRow::to_domain)
        .transpose()
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        let id = review_id.value().to_string();
        sqlx::query("DELETE FROM reviews WHERE id = $1")
            .bind(&id)
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
impl DiaryRepository for PostgresRepository {
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
            .map(DiaryRow::to_domain)
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

        // Dynamic param counter
        let mut p: i32 = 0;
        let mut next_param = || {
            p += 1;
            format!("${}", p)
        };

        let mut where_parts = vec!["1=1".to_string()];

        if has_search {
            let pn = next_param();
            where_parts.push(format!("m.title ILIKE '%' || {} || '%'", pn));
        }

        if let Some(f) = following {
            let local_params: Vec<String> =
                f.local_user_ids.iter().map(|_| next_param()).collect();
            let remote_params: Vec<String> =
                f.remote_actor_urls.iter().map(|_| next_param()).collect();

            let local_in = if local_params.is_empty() {
                "(SELECT NULL::text WHERE false)".to_string()
            } else {
                local_params.join(", ")
            };
            let remote_in = if remote_params.is_empty() {
                "(SELECT NULL::text WHERE false)".to_string()
            } else {
                remote_params.join(", ")
            };
            where_parts.push(format!(
                "(r.user_id IN ({}) OR r.remote_actor_url IN ({}))",
                local_in, remote_in
            ));
        }

        let limit_param = next_param();
        let offset_param = next_param();

        let order_clause = match sort_by {
            FeedSortBy::Date => "r.watched_at DESC",
            FeedSortBy::DateAsc => "r.watched_at ASC",
            FeedSortBy::Rating => "r.rating DESC, r.watched_at DESC",
            FeedSortBy::RatingAsc => "r.rating ASC, r.watched_at ASC",
        };

        let where_clause = where_parts.join(" AND ");

        // Reset counter for count query (reuse same where_clause string but re-bind)
        // We need a separate counter for count SQL — but since where_clause is already built
        // with the right $N references, both queries share it.
        let count_sql = format!(
            "SELECT COUNT(*) FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE {}",
            where_clause
        );

        let select_sql = format!(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url,
                    COALESCE(u.email, r.remote_actor_url) AS user_email
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             LEFT JOIN users u ON u.id = r.user_id
             WHERE {}
             ORDER BY {}
             LIMIT {} OFFSET {}",
            where_clause, order_clause, limit_param, offset_param
        );

        // Bind helper closure — binds search + following params in order
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
        let total = count_q
            .fetch_one(&self.pool)
            .await
            .map_err(Self::map_err)?;

        let rows_q = bind_filter_params!(sqlx::query_as::<_, FeedRow>(&select_sql));
        let rows = rows_q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?;

        let items = rows
            .into_iter()
            .map(FeedRow::to_domain)
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
             FROM movies WHERE id = $1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", id_str)))?
        .to_domain()?;

        let viewings = sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment,
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    remote_actor_url
             FROM reviews WHERE movie_id = $1 ORDER BY watched_at ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(ReviewRow::to_domain)
        .collect::<Result<Vec<_>, _>>()?;

        Ok(ReviewHistory::new(movie, viewings))
    }

    async fn get_user_history(&self, user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1
             ORDER BY r.watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter().map(DiaryRow::to_domain).collect()
    }

    async fn get_movie_stats(&self, movie_id: &MovieId) -> Result<MovieStats, DomainError> {
        let id_str = movie_id.value().to_string();
        sqlx::query_as::<_, MovieStatsRow>(
            "SELECT
                COUNT(*) AS total_count,
                AVG(CAST(rating AS FLOAT)) AS avg_rating,
                COUNT(CASE WHEN remote_actor_url IS NOT NULL THEN 1 END) AS federated_count,
                COUNT(CASE WHEN rating = 1 THEN 1 END) AS rating_1,
                COUNT(CASE WHEN rating = 2 THEN 1 END) AS rating_2,
                COUNT(CASE WHEN rating = 3 THEN 1 END) AS rating_3,
                COUNT(CASE WHEN rating = 4 THEN 1 END) AS rating_4,
                COUNT(CASE WHEN rating = 5 THEN 1 END) AS rating_5
             FROM reviews WHERE movie_id = $1",
        )
        .bind(id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)
        .map(MovieStatsRow::to_domain)
    }

    async fn get_movie_social_feed(
        &self,
        movie_id: &MovieId,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        let id_str = movie_id.value().to_string();
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reviews WHERE movie_id = $1",
        )
        .bind(&id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let rows = sqlx::query_as::<_, FeedRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url,
                    CASE WHEN r.remote_actor_url IS NOT NULL THEN r.remote_actor_url
                         WHEN u.email IS NOT NULL THEN u.email
                         ELSE r.user_id END AS user_email
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             LEFT JOIN users u ON u.id = r.user_id
             WHERE r.movie_id = $1
             ORDER BY r.watched_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(&id_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let items = rows
            .into_iter()
            .map(FeedRow::to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reviews WHERE remote_actor_url IS NULL"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(count as u64)
    }
}

#[async_trait]
impl StatsRepository for PostgresRepository {
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
            sqlx::query_as::<_, MonthlyRatingRow>(
                "SELECT to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') AS month,
                        AVG(rating::float) AS avg_rating,
                        COUNT(*) AS count
                 FROM reviews
                 WHERE user_id = $1 AND watched_at >= NOW() - INTERVAL '12 months'
                 GROUP BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM')
                 ORDER BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') ASC"
            )
            .bind(&uid)
            .fetch_all(&self.pool),
            sqlx::query_as::<_, DirectorCountRow>(
                "SELECT m.director AS director, COUNT(*) AS count
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND m.director IS NOT NULL
                 GROUP BY m.director
                 ORDER BY COUNT(*) DESC
                 LIMIT 5"
            )
            .bind(&uid)
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

pub async fn wire(database_url: &str) -> anyhow::Result<(
    sqlx::PgPool,
    std::sync::Arc<dyn domain::ports::MovieRepository>,
    std::sync::Arc<dyn domain::ports::ReviewRepository>,
    std::sync::Arc<dyn domain::ports::DiaryRepository>,
    std::sync::Arc<dyn domain::ports::StatsRepository>,
    std::sync::Arc<dyn domain::ports::UserRepository>,
    std::sync::Arc<dyn domain::ports::ImportSessionRepository>,
    std::sync::Arc<dyn domain::ports::ImportProfileRepository>,
    std::sync::Arc<dyn domain::ports::MovieProfileRepository>,
)> {
    use anyhow::Context;

    let pool = sqlx::PgPool::connect(database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    let repo = std::sync::Arc::new(PostgresRepository::new(pool.clone()));
    repo.migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Database migration failed")?;

    let import_session_repo = std::sync::Arc::new(PostgresImportSessionRepository::new(pool.clone()));
    let import_profile_repo = std::sync::Arc::new(PostgresImportProfileRepository::new(pool.clone()));
    let movie_profile_repo = std::sync::Arc::new(PostgresMovieProfileRepository::new(pool.clone()));

    Ok((
        pool.clone(),
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::clone(&repo) as _,
        std::sync::Arc::new(PostgresUserRepository::new(pool)) as _,
        import_session_repo as _,
        import_profile_repo as _,
        movie_profile_repo as _,
    ))
}
