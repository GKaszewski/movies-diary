use crate::shared::{
    movie_row_to_domain, watchlist_entry_to_domain, watchlist_with_movie_to_domain,
};
use domain::{errors::DomainError, models::WatchlistWithMovie};

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
