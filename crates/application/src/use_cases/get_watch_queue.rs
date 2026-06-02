use domain::{errors::DomainError, models::WatchEvent, value_objects::UserId};

use crate::{context::AppContext, queries::GetWatchQueueQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetWatchQueueQuery,
) -> Result<Vec<WatchEvent>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    ctx.watch_event_repository.list_pending(&user_id).await
}
