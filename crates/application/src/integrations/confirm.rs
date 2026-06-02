use domain::{
    errors::DomainError,
    models::WatchEventStatus,
    value_objects::{UserId, WatchEventId},
};

use crate::{
    context::AppContext,
    diary::commands::{LogReviewCommand, MovieInput},
    diary::log_review,
    integrations::commands::ConfirmWatchEventsCommand,
};

pub async fn execute(ctx: &AppContext, cmd: ConfirmWatchEventsCommand) -> Result<u32, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let mut confirmed = 0u32;

    for c in cmd.confirmations {
        let event_id = WatchEventId::from_uuid(c.watch_event_id);
        let event = ctx
            .repos
            .watch_event
            .get_by_id(&event_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("WatchEvent {}", c.watch_event_id)))?;

        if event.user_id() != &user_id {
            return Err(DomainError::Forbidden("not your watch event".into()));
        }

        let input = if let Some(movie_id) = event.movie_id() {
            MovieInput {
                movie_id: Some(movie_id.value()),
                external_metadata_id: None,
                manual_title: None,
                manual_release_year: None,
                manual_director: None,
            }
        } else {
            MovieInput {
                movie_id: None,
                external_metadata_id: event.external_metadata_id().map(String::from),
                manual_title: Some(event.title().to_string()),
                manual_release_year: event.year(),
                manual_director: None,
            }
        };

        let review_cmd = LogReviewCommand {
            user_id: cmd.user_id,
            input,
            rating: c.rating,
            comment: c.comment,
            watched_at: *event.watched_at(),
        };

        log_review::execute(ctx, review_cmd).await?;

        ctx.repos
            .watch_event
            .update_status(&event_id, WatchEventStatus::Confirmed)
            .await?;

        confirmed += 1;
    }

    Ok(confirmed)
}
