use crate::shared::user_summary_to_domain;
use domain::errors::DomainError;

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
