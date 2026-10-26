use crate::shared::{movie_row_to_domain, movie_stats_to_domain, review_row_to_domain};
use domain::{
    errors::DomainError,
    models::{DiaryEntry, FeedEntry},
};

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
    pub spoilers: bool,
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
            self.spoilers,
        )?;
        Ok(DiaryEntry::new(movie, review))
    }
}

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
    pub spoilers: bool,
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
            self.spoilers,
        )?;
        let diary = DiaryEntry::new(movie, review);
        Ok(FeedEntry::new(diary, self.user_email))
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
