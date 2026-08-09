use domain::{
    errors::DomainError,
    value_objects::{MovieId, UserId},
};

use crate::watchlist::deps::IsOnWatchlistDeps;
use crate::watchlist::queries::IsOnWatchlistQuery;

pub async fn execute(
    deps: &IsOnWatchlistDeps,
    query: IsOnWatchlistQuery,
) -> Result<bool, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let movie_id = MovieId::from_uuid(query.movie_id);
    deps.watchlist.contains(&user_id, &movie_id).await
}

#[cfg(test)]
#[path = "tests/is_on.rs"]
mod tests;
