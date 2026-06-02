use domain::{
    errors::DomainError,
    models::WatchEventStatus,
    value_objects::{UserId, WatchEventId},
};

use crate::{context::AppContext, integrations::commands::DismissWatchEventsCommand};

pub async fn execute(ctx: &AppContext, cmd: DismissWatchEventsCommand) -> Result<u32, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    if cmd.event_ids.is_empty() {
        return Ok(0);
    }

    let ids: Vec<WatchEventId> = cmd
        .event_ids
        .iter()
        .map(|id| WatchEventId::from_uuid(*id))
        .collect();

    let events = ctx.repos.watch_event.get_by_ids(&ids).await?;

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

    let count = ctx
        .repos
        .watch_event
        .update_status_batch(&ids, WatchEventStatus::Dismissed)
        .await?;

    Ok(count as u32)
}
