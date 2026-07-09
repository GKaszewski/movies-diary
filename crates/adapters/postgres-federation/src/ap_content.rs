use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, Goal, GoalType, Movie, PersistedReview, Review, ReviewSource, WatchlistEntry,
        WatchlistWithMovie,
    },
    ports::LocalApContentQuery,
    value_objects::{
        Comment, ExternalMetadataId, GoalId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
        ReviewId, UserId, WatchlistEntryId,
    },
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresApContentQuery {
    pool: PgPool,
}

impl PostgresApContentQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

// ── Local row types ──────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Result<Uuid, DomainError> {
    Uuid::parse_str(s)
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid UUID '{}': {}", s, e)))
}

fn parse_datetime(s: &str) -> Result<chrono::NaiveDateTime, DomainError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid datetime '{}': {}", s, e)))
}

#[derive(sqlx::FromRow)]
struct MovieRow {
    id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
}

impl MovieRow {
    fn into_domain(self) -> Result<Movie, DomainError> {
        let id = MovieId::from_uuid(parse_uuid(&self.id)?);
        let external_metadata_id = self
            .external_metadata_id
            .map(ExternalMetadataId::new)
            .transpose()?;
        let title = MovieTitle::new(self.title)?;
        let release_year = ReleaseYear::new(self.release_year as u16)?;
        let poster_path = self.poster_path.map(PosterPath::new).transpose()?;
        Ok(Movie::from_persistence(
            id,
            external_metadata_id,
            title,
            release_year,
            self.director,
            poster_path,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct ReviewRow {
    id: String,
    movie_id: String,
    user_id: String,
    rating: i64,
    comment: Option<String>,
    watched_at: String,
    created_at: String,
    remote_actor_url: Option<String>,
}

impl ReviewRow {
    fn into_domain(self) -> Result<Review, DomainError> {
        let id = ReviewId::from_uuid(parse_uuid(&self.id)?);
        let movie_id = MovieId::from_uuid(parse_uuid(&self.movie_id)?);
        let user_id = UserId::from_uuid(parse_uuid(&self.user_id)?);
        let rating = Rating::new(self.rating as u8)?;
        let comment = self.comment.map(Comment::new).transpose()?;
        let watched_at = parse_datetime(&self.watched_at)?;
        let created_at = parse_datetime(&self.created_at)?;
        let source = match self.remote_actor_url {
            None => ReviewSource::Local,
            Some(url) => ReviewSource::Remote { actor_url: url },
        };
        Ok(Review::from_persistence(PersistedReview {
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            source,
            watch_medium: None,
        }))
    }
}

#[derive(sqlx::FromRow)]
struct DiaryRow {
    id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
    review_id: String,
    movie_id: String,
    user_id: String,
    rating: i64,
    comment: Option<String>,
    watched_at: String,
    created_at: String,
    remote_actor_url: Option<String>,
}

impl DiaryRow {
    fn into_domain(self) -> Result<DiaryEntry, DomainError> {
        let movie = MovieRow {
            id: self.id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .into_domain()?;
        let review = ReviewRow {
            id: self.review_id,
            movie_id: self.movie_id,
            user_id: self.user_id,
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            created_at: self.created_at,
            remote_actor_url: self.remote_actor_url,
        }
        .into_domain()?;
        Ok(DiaryEntry::new(movie, review))
    }
}

fn row_to_goal(r: &sqlx::postgres::PgRow) -> Result<Goal, DomainError> {
    let id_str: String = r
        .try_get("id")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read goal id: {e}")))?;
    let user_id_str: String = r
        .try_get("user_id")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read user_id: {e}")))?;
    let year: i64 = r
        .try_get("year")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read year: {e}")))?;
    let target: i64 = r.try_get("target_count").map_err(|e| {
        DomainError::InfrastructureError(format!("Failed to read target_count: {e}"))
    })?;
    let goal_type_str: String = r
        .try_get("goal_type")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read goal_type: {e}")))?;
    let created_at_str: String = r
        .try_get("created_at")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read created_at: {e}")))?;

    let id = GoalId::from_uuid(parse_uuid(&id_str)?);
    let user_id = UserId::from_uuid(parse_uuid(&user_id_str)?);
    let goal_type: GoalType = goal_type_str.parse()?;
    let created_at = parse_datetime(&created_at_str)?;

    Ok(Goal::from_persistence(
        id,
        user_id,
        year as u16,
        target as u32,
        goal_type,
        created_at,
    ))
}

async fn count_reviews_in_year(
    pool: &PgPool,
    user_id: &UserId,
    year: u16,
) -> Result<u32, DomainError> {
    let uid = user_id.value().to_string();
    let start = format!("{year}-01-01 00:00:00");
    let end = format!("{}-01-01 00:00:00", year + 1);

    let count: i64 = sqlx::query(
        "SELECT COUNT(*) FROM reviews \
         WHERE user_id = $1 \
         AND watched_at >= $2::timestamptz \
         AND watched_at < $3::timestamptz \
         AND remote_actor_url IS NULL",
    )
    .bind(&uid)
    .bind(&start)
    .bind(&end)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    })?
    .try_get(0)
    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

    Ok(count as u32)
}

// ── LocalApContentQuery impl ─────────────────────────────────────────────────

#[async_trait]
impl LocalApContentQuery for PostgresApContentQuery {
    async fn get_local_reviews_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1 AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query(
            "SELECT w.id, w.user_id, w.movie_id,
                    to_char(w.added_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS added_at,
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year,
                    m.director, m.poster_path
             FROM watchlist_entries w
             JOIN movies m ON m.id = w.movie_id
             WHERE w.user_id = $1
             ORDER BY w.added_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter()
            .map(|row| {
                let entry = WatchlistEntry {
                    id: WatchlistEntryId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    user_id: UserId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("user_id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    movie_id: MovieId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("movie_id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    added_at: parse_datetime(
                        &row.try_get::<String, _>("added_at")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?,
                };
                let movie = MovieRow {
                    id: row
                        .try_get("m_id")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    external_metadata_id: row
                        .try_get("external_metadata_id")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    title: row
                        .try_get("title")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    release_year: row
                        .try_get("release_year")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    director: row
                        .try_get("director")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    poster_path: row
                        .try_get("poster_path")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                }
                .into_domain()?;
                Ok(WatchlistWithMovie { entry, movie })
            })
            .collect()
    }

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let mid = movie_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = $1 AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&mid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
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
        .map(ReviewRow::into_domain)
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
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movie_by_external_metadata_id(
        &self,
        external_id: &str,
    ) -> Result<Option<Movie>, DomainError> {
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = $1",
        )
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE remote_actor_url IS NULL")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err)?;
        Ok(count as u64)
    }

    async fn get_local_reviews_page(
        &self,
        user_id: &UserId,
        before: Option<chrono::NaiveDateTime>,
        limit: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let limit_i64 = limit as i64;

        let rows = if let Some(before_ts) = before {
            let ts = before_ts.format("%Y-%m-%d %H:%M:%S").to_string();
            sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                        to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                        to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                        r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND r.remote_actor_url IS NULL AND r.watched_at < $2::timestamptz
                 ORDER BY r.watched_at DESC
                 LIMIT $3",
            )
            .bind(&uid)
            .bind(&ts)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?
        } else {
            sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                        to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                        to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                        r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND r.remote_actor_url IS NULL
                 ORDER BY r.watched_at DESC
                 LIMIT $2",
            )
            .bind(&uid)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?
        };
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    async fn get_goal_with_progress(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<(Goal, u32)>, DomainError> {
        let uid = user_id.value().to_string();
        let y = year as i64;

        let row = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, \
             to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM goals WHERE user_id = $1 AND year = $2",
        )
        .bind(&uid)
        .bind(y)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let Some(r) = row else { return Ok(None) };

        let goal = row_to_goal(&r)?;
        let count = count_reviews_in_year(&self.pool, user_id, year).await?;

        Ok(Some((goal, count)))
    }

    async fn list_goals_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, \
             to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM goals WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.iter().map(row_to_goal).collect()
    }
}
