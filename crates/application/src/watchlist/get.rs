use domain::{
    errors::DomainError,
    models::{
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    value_objects::UserId,
};

use crate::{context::AppContext, watchlist::queries::GetWatchlistQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetWatchlistQuery,
) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let page = PageParams::new(query.limit, query.offset)?;
    ctx.repos.watchlist.get_for_user(&user_id, &page).await
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
