use domain::{
    errors::DomainError,
    value_objects::{MovieId, UserId},
};

use crate::{context::AppContext, watchlist::queries::IsOnWatchlistQuery};

pub async fn execute(ctx: &AppContext, query: IsOnWatchlistQuery) -> Result<bool, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let movie_id = MovieId::from_uuid(query.movie_id);
    ctx.repos.watchlist.contains(&user_id, &movie_id).await
}
