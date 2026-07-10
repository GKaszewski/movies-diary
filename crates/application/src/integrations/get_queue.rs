use std::sync::Arc;

use domain::{
    errors::DomainError, models::WatchEvent, ports::WatchEventQuery, value_objects::UserId,
};

use crate::integrations::queries::GetWatchQueueQuery;

pub async fn execute(
    watch_event_query: Arc<dyn WatchEventQuery>,
    query: GetWatchQueueQuery,
) -> Result<Vec<WatchEvent>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    watch_event_query.list_pending(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_queue.rs"]
mod tests;
