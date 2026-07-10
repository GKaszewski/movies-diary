use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, Movie, PersistedReview, Review, ReviewSource, WatchlistEntry,
        WatchlistWithMovie,
    },
    ports::LocalApContentQuery,
    value_objects::{
        Comment, ExternalMetadataId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
        ReviewId, UserId, WatchlistEntryId,
    },
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteApContentQuery {
    pool: SqlitePool,
}

impl SqliteApContentQuery {
    pub fn new(pool: SqlitePool) -> Self {
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
    watch_medium: Option<String>,
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
        let watch_medium = self.watch_medium.map(|s| s.parse()).transpose()?;
        Ok(Review::from_persistence(PersistedReview {
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            source,
            watch_medium,
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
    watch_medium: Option<String>,
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
            watch_medium: self.watch_medium,
        }
        .into_domain()?;
        Ok(DiaryEntry::new(movie, review))
    }
}

#[derive(sqlx::FromRow)]
struct WatchlistRow {
    id: String,
    user_id: String,
    movie_id: String,
    added_at: String,
    m_id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
}

impl WatchlistRow {
    fn into_domain(self) -> Result<WatchlistWithMovie, DomainError> {
        let entry = WatchlistEntry {
            id: WatchlistEntryId::from_uuid(parse_uuid(&self.id)?),
            user_id: UserId::from_uuid(parse_uuid(&self.user_id)?),
            movie_id: MovieId::from_uuid(parse_uuid(&self.movie_id)?),
            added_at: parse_datetime(&self.added_at)?,
        };
        let movie = MovieRow {
            id: self.m_id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .into_domain()?;
        Ok(WatchlistWithMovie { entry, movie })
    }
}

// ── LocalApContentQuery impl ─────────────────────────────────────────────────

#[async_trait]
impl LocalApContentQuery for SqliteApContentQuery {
    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError> {
        let uid = user_id.value().to_string();
        let rows: Vec<WatchlistRow> = sqlx::query_as(
            "SELECT w.id, w.user_id, w.movie_id, w.added_at,
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year,
                    m.director, m.poster_path
             FROM watchlist_entries w
             JOIN movies m ON m.id = w.movie_id
             WHERE w.user_id = ?
             ORDER BY w.added_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(WatchlistRow::into_domain).collect()
    }

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let mid = movie_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = ? AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&mid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
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
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ? AND r.remote_actor_url IS NULL AND r.watched_at < ?
                 ORDER BY r.watched_at DESC
                 LIMIT ?",
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
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ? AND r.remote_actor_url IS NULL
                 ORDER BY r.watched_at DESC
                 LIMIT ?",
            )
            .bind(&uid)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?
        };
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }
}
