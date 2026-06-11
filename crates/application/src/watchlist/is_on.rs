use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::WatchlistRepository,
    value_objects::{MovieId, UserId},
};

use crate::watchlist::queries::IsOnWatchlistQuery;

pub async fn execute(
    watchlist: Arc<dyn WatchlistRepository>,
    query: IsOnWatchlistQuery,
) -> Result<bool, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let movie_id = MovieId::from_uuid(query.movie_id);
    watchlist.contains(&user_id, &movie_id).await
}

#[cfg(test)]
#[path = "tests/is_on.rs"]
mod tests;
