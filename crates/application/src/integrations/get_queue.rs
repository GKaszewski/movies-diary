use domain::{errors::DomainError, models::WatchEvent, value_objects::UserId};

use crate::{context::AppContext, integrations::queries::GetWatchQueueQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetWatchQueueQuery,
) -> Result<Vec<WatchEvent>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    ctx.repos.watch_event.list_pending(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_queue.rs"]
mod tests;
