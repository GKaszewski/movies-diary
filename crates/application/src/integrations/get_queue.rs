use std::sync::Arc;

use domain::{
    errors::DomainError, models::WatchEvent, ports::WatchEventRepository, value_objects::UserId,
};

use crate::integrations::queries::GetWatchQueueQuery;

pub async fn execute(
    watch_event: Arc<dyn WatchEventRepository>,
    query: GetWatchQueueQuery,
) -> Result<Vec<WatchEvent>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    watch_event.list_pending(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_queue.rs"]
mod tests;
