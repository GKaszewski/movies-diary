use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::{
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::WatchlistRepository,
    value_objects::UserId,
};

use crate::watchlist::queries::GetWatchlistQuery;

pub async fn execute(
    watchlist: Arc<dyn WatchlistRepository>,
    query: GetWatchlistQuery,
) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let page = PageParams::new(query.limit, query.offset)?;
    watchlist.get_for_user(&user_id, &page).await
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
