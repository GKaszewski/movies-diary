use adapter_common::{
    movie_row_to_domain, movie_stats_to_domain, movie_summary_to_domain, review_row_to_domain,
    user_summary_to_domain, watchlist_entry_to_domain, watchlist_with_movie_to_domain,
};
use domain::{
    errors::DomainError,
    models::{DiaryEntry, FeedEntry, Movie, MovieSummary, Review, WatchlistWithMovie},
};

#[derive(sqlx::FromRow)]
pub(crate) struct MovieRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
}

impl MovieRow {
    pub fn into_domain(self) -> Result<Movie, DomainError> {
        movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MovieSummaryRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Option<String>,
    pub runtime_minutes: Option<i64>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub collection_name: Option<String>,
}

impl MovieSummaryRow {
    pub fn into_domain(self) -> Result<MovieSummary, DomainError> {
        let movie = movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )?;
        let genres = self
            .genres
            .map(|g| g.split(',').map(str::to_string).collect())
            .unwrap_or_default();
        Ok(movie_summary_to_domain(
            movie,
            genres,
            self.runtime_minutes,
            self.original_language,
            self.overview,
            self.collection_name,
        ))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct ReviewRow {
    pub id: String,
    pub movie_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: Option<String>,
    pub watched_at: String,
    pub created_at: String,
    pub remote_actor_url: Option<String>,
    pub watch_medium: Option<String>,
}

impl ReviewRow {
    pub fn into_domain(self) -> Result<Review, DomainError> {
        review_row_to_domain(
            self.id,
            self.movie_id,
            self.user_id,
            self.rating,
            self.comment,
            self.watched_at,
            self.created_at,
            self.remote_actor_url,
            self.watch_medium,
        )
    }
}

// Used by query_diary JOIN — r.id aliased to review_id to avoid ambiguity with m.id
#[derive(sqlx::FromRow)]
pub(crate) struct DiaryRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub review_id: String,
    pub movie_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: Option<String>,
    pub watched_at: String,
    pub created_at: String,
    pub remote_actor_url: Option<String>,
    pub watch_medium: Option<String>,
}

impl DiaryRow {
    pub fn into_domain(self) -> Result<DiaryEntry, DomainError> {
        let movie = movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )?;
        let review = review_row_to_domain(
            self.review_id,
            self.movie_id,
            self.user_id,
            self.rating,
            self.comment,
            self.watched_at,
            self.created_at,
            self.remote_actor_url,
            self.watch_medium,
        )?;
        Ok(DiaryEntry::new(movie, review))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MovieStatsRow {
    pub total_count: i64,
    pub avg_rating: Option<f64>,
    pub federated_count: i64,
    pub rating_1: i64,
    pub rating_2: i64,
    pub rating_3: i64,
    pub rating_4: i64,
    pub rating_5: i64,
}

impl MovieStatsRow {
    pub fn into_domain(self) -> domain::models::MovieStats {
        movie_stats_to_domain(
            self.total_count,
            self.avg_rating,
            self.federated_count,
            [
                self.rating_1,
                self.rating_2,
                self.rating_3,
                self.rating_4,
                self.rating_5,
            ],
        )
    }
}

// Like DiaryRow but includes user_email from JOIN with users table
#[derive(sqlx::FromRow)]
pub(crate) struct FeedRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub review_id: String,
    pub movie_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: Option<String>,
    pub watched_at: String,
    pub created_at: String,
    pub remote_actor_url: Option<String>,
    pub watch_medium: Option<String>,
    pub user_email: String,
}

impl FeedRow {
    pub fn into_domain(self) -> Result<FeedEntry, DomainError> {
        let movie = movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )?;
        let review = review_row_to_domain(
            self.review_id,
            self.movie_id,
            self.user_id,
            self.rating,
            self.comment,
            self.watched_at,
            self.created_at,
            self.remote_actor_url,
            self.watch_medium,
        )?;
        let diary = DiaryEntry::new(movie, review);
        Ok(FeedEntry::new(diary, self.user_email))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct UserSummaryRow {
    pub id: String,
    pub email: String,
    pub username: String,
    pub display_name: Option<String>,
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub avatar_path: Option<String>,
}

impl UserSummaryRow {
    pub fn into_domain(self) -> Result<domain::models::UserSummary, DomainError> {
        user_summary_to_domain(
            self.id,
            self.email,
            self.username,
            self.display_name,
            self.total_movies,
            self.avg_rating,
            self.avatar_path,
        )
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct UserTotalsRow {
    pub total: i64,
    pub avg_rating: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct DirectorCountRow {
    pub director: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct MonthlyRatingRow {
    pub month: String,
    pub avg_rating: f64,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct WatchlistRow {
    pub id: String,
    pub user_id: String,
    pub movie_id: String,
    pub added_at: String,
    pub m_id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
}

impl WatchlistRow {
    pub fn into_domain(self) -> Result<WatchlistWithMovie, DomainError> {
        let entry = watchlist_entry_to_domain(self.id, self.user_id, self.movie_id, self.added_at)?;
        let movie = movie_row_to_domain(
            self.m_id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )?;
        Ok(watchlist_with_movie_to_domain(entry, movie))
    }
}
