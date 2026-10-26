use crate::shared::review_row_to_domain;
use domain::{errors::DomainError, models::Review};

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
    pub spoilers: bool,
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
            self.spoilers,
        )
    }
}
