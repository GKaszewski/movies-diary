use domain::{errors::DomainError, models::RemoteWatchlistEntry};

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, uuid: uuid::Uuid) -> Result<Vec<RemoteWatchlistEntry>, DomainError> {
    ctx.remote_watchlist_repository.get_by_derived_uuid(uuid).await
}
