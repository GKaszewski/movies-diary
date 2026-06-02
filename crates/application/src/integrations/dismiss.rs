use domain::{
    errors::DomainError,
    models::WatchEventStatus,
    value_objects::{UserId, WatchEventId},
};

use crate::{context::AppContext, integrations::commands::DismissWatchEventsCommand};

pub async fn execute(ctx: &AppContext, cmd: DismissWatchEventsCommand) -> Result<u32, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let mut dismissed = 0u32;

    for id in cmd.event_ids {
        let event_id = WatchEventId::from_uuid(id);
        let event = ctx
            .repos
            .watch_event
            .get_by_id(&event_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("WatchEvent {id}")))?;

        if event.user_id() != &user_id {
            return Err(DomainError::Unauthorized("not your watch event".into()));
        }

        ctx.repos
            .watch_event
            .update_status(&event_id, WatchEventStatus::Dismissed)
            .await?;

        dismissed += 1;
    }

    Ok(dismissed)
}
