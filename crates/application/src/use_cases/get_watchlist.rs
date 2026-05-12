use domain::{
    errors::DomainError,
    models::{WatchlistWithMovie, collections::{PageParams, Paginated}},
    value_objects::UserId,
};

use crate::{context::AppContext, queries::GetWatchlistQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetWatchlistQuery,
) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let page = PageParams::new(query.limit, query.offset)?;
    ctx.watchlist_repository.get_for_user(&user_id, &page).await
}
