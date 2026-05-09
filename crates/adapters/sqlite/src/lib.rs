use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, DirectorStat, FeedEntry, Movie, MonthlyRating,
        Review, ReviewHistory, ReviewSource, SortDirection, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::MovieRepository,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear, ReviewId, UserId},
};
use sqlx::SqlitePool;

mod migrations;
mod models;
mod users;

use models::{
    DiaryRow, DirectorCountRow, FeedRow, MonthlyRatingRow, MovieRow, ReviewRow,
    UserTotalsRow, datetime_to_str,
};

pub use users::SqliteUserRepository;

fn format_year_month(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 { return ym.to_string(); }
    let year = parts[0].get(2..).unwrap_or(parts[0]);
    let month = match parts[1] {
        "01" => "Jan", "02" => "Feb", "03" => "Mar", "04" => "Apr",
        "05" => "May", "06" => "Jun", "07" => "Jul", "08" => "Aug",
        "09" => "Sep", "10" => "Oct", "11" => "Nov", "12" => "Dec",
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
            Some(id) => {
                sqlx::query_scalar!("SELECT COUNT(*) FROM reviews WHERE movie_id = ?", id)
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
        match sort {
            // ByRatingDesc only applies to user-scoped queries; falls back to date sort here
            SortDirection::Descending | SortDirection::ByRatingDesc => sqlx::query_as!(
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
            // ByRatingDesc only applies to user-scoped queries; falls back to date sort here
            SortDirection::Descending | SortDirection::ByRatingDesc => sqlx::query_as!(
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

    async fn count_user_diary_entries(&self, user_id: &str) -> Result<i64, DomainError> {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM reviews WHERE user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn fetch_user_diary_rows_by_watched(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        sqlx::query_as!(
            DiaryRow,
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?
             ORDER BY r.watched_at DESC
             LIMIT ? OFFSET ?",
            user_id, limit, offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn fetch_user_diary_rows_by_rating(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        sqlx::query_as!(
            DiaryRow,
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ?
             ORDER BY r.rating DESC, r.watched_at DESC
             LIMIT ? OFFSET ?",
            user_id, limit, offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)
    }

    async fn count_feed_entries(&self) -> Result<i64, DomainError> {
        sqlx::query_scalar!("SELECT COUNT(*) FROM reviews")
            .fetch_one(&self.pool)
            .await
            .map_err(Self::map_err)
    }

    async fn fetch_feed_rows(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FeedRow>, DomainError> {
        sqlx::query_as!(
            FeedRow,
            r#"SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url,
                    COALESCE(u.email, r.remote_actor_url) AS "user_email!: String"
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             LEFT JOIN users u ON u.id = r.user_id
             ORDER BY r.watched_at DESC
             LIMIT ? OFFSET ?"#,
            limit, offset
        )
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
        .map(MovieRow::to_domain)
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

    async fn query_diary(&self, filter: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> {
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
                match &filter.sort_by {
                    SortDirection::ByRatingDesc => tokio::try_join!(
                        self.count_user_diary_entries(&uid_str),
                        self.fetch_user_diary_rows_by_rating(&uid_str, limit, offset)
                    )?,
                    _ => tokio::try_join!(
                        self.count_user_diary_entries(&uid_str),
                        self.fetch_user_diary_rows_by_watched(&uid_str, limit, offset)
                    )?,
                }
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
        .map(ReviewRow::to_domain)
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

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query!("DELETE FROM movies WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
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
        .to_domain()?;

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
        .map(ReviewRow::to_domain)
        .collect::<Result<Vec<_>, _>>()?;

        Ok(ReviewHistory::new(movie, viewings))
    }

    async fn query_activity_feed(
        &self,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        let (total, rows) = tokio::try_join!(
            self.count_feed_entries(),
            self.fetch_feed_rows(limit, offset)
        )?;

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

        rows.into_iter().map(DiaryRow::to_domain).collect()
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
            .map(|d| DirectorStat { director: d.director, count: d.count })
            .collect();

        Ok(UserTrends { monthly_ratings, top_directors, max_director_count })
    }
}
