use domain::{errors::DomainError, models::WatchEvent, value_objects::UserId};

use crate::integrations::{deps::GetWatchQueueDeps, queries::GetWatchQueueQuery};

pub async fn execute(
    deps: &GetWatchQueueDeps,
    query: GetWatchQueueQuery,
) -> Result<Vec<WatchEvent>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.watch_event_query.list_pending(&user_id).await
}
