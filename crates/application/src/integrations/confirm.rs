use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::WatchEventStatus,
    ports::{WatchEventCommand, WatchEventQuery},
    value_objects::{UserId, WatchEventId},
};

use crate::{
    diary::commands::{LogReviewCommand, MovieInput},
    integrations::commands::ConfirmWatchEventsCommand,
    ports::ReviewLogger,
};

pub async fn execute(
    watch_event_command: Arc<dyn WatchEventCommand>,
    watch_event_query: Arc<dyn WatchEventQuery>,
    review_logger: Arc<dyn ReviewLogger>,
    cmd: ConfirmWatchEventsCommand,
) -> Result<u32, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let mut confirmed = 0u32;

    for c in cmd.confirmations {
        let event_id = WatchEventId::from_uuid(c.watch_event_id);
        let event = watch_event_query
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
            watch_medium: Some(domain::value_objects::WatchMedium::MediaServer),
        };

        review_logger.log_review(review_cmd).await?;

        watch_event_command
            .update_status(&event_id, WatchEventStatus::Confirmed)
            .await?;

        confirmed += 1;
    }

    Ok(confirmed)
}

#[cfg(test)]
#[path = "tests/confirm.rs"]
mod tests;
