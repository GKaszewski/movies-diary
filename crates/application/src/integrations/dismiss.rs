use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::WatchEventStatus,
    ports::{WatchEventCommand, WatchEventQuery},
    value_objects::{UserId, WatchEventId},
};

use crate::integrations::commands::DismissWatchEventsCommand;

pub async fn execute(
    watch_event_command: Arc<dyn WatchEventCommand>,
    watch_event_query: Arc<dyn WatchEventQuery>,
    cmd: DismissWatchEventsCommand,
) -> Result<u32, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    if cmd.event_ids.is_empty() {
        return Ok(0);
    }

    let ids: Vec<WatchEventId> = cmd
        .event_ids
        .iter()
        .map(|id| WatchEventId::from_uuid(*id))
        .collect();

    let events = watch_event_query.get_by_ids(&ids).await?;

    if events.len() != ids.len() {
        return Err(DomainError::NotFound(
            "one or more WatchEvents not found".into(),
        ));
    }
    for event in &events {
        if event.user_id() != &user_id {
            return Err(DomainError::Forbidden("not your watch event".into()));
        }
    }

    let count = watch_event_command
        .update_status_batch(&ids, WatchEventStatus::Dismissed)
        .await?;

    Ok(count as u32)
}

#[cfg(test)]
#[path = "tests/dismiss.rs"]
mod tests;
