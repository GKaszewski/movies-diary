use domain::{
    errors::DomainError,
    models::{
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    value_objects::UserId,
};

use crate::watchlist::deps::GetWatchlistDeps;
use crate::watchlist::queries::GetWatchlistQuery;

pub async fn execute(
    deps: &GetWatchlistDeps,
    query: GetWatchlistQuery,
) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let page = PageParams::new(query.limit, query.offset)?;
    deps.watchlist.get_for_user(&user_id, &page).await
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
