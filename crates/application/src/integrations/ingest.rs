use chrono::Duration;
use domain::{
    errors::DomainError, events::DomainEvent, models::WatchEvent, ports::MediaServerParser,
};

use crate::{context::AppContext, integrations::commands::IngestWatchEventCommand};

pub async fn execute(
    ctx: &AppContext,
    cmd: IngestWatchEventCommand,
    parser: &dyn MediaServerParser,
) -> Result<(), DomainError> {
    let token_hash = super::generate_token::hash_token(&cmd.token);
    let webhook_token = ctx
        .repos
        .webhook_token
        .find_by_token_hash(&token_hash)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("invalid webhook token".into()))?;

    let _ = ctx
        .repos
        .webhook_token
        .touch_last_used(webhook_token.id())
        .await;

    let parsed = match parser.parse_playback_event(&cmd.raw_payload)? {
        Some(event) => event,
        None => return Ok(()),
    };

    let external_metadata_id = parsed.tmdb_id.or(parsed.imdb_id);
    let user_id = webhook_token.user_id().clone();

    if let Some(ref ext_id) = external_metadata_id {
        let one_hour_ago = chrono::Utc::now().naive_utc() - Duration::hours(1);
        if ctx
            .repos
            .watch_event
            .find_duplicate(&user_id, ext_id, one_hour_ago)
            .await?
        {
            return Ok(());
        }
    }

    let watched_at = chrono::Utc::now().naive_utc();
    let event = WatchEvent::new(
        user_id,
        parsed.title,
        parsed.year,
        external_metadata_id,
        cmd.source,
        watched_at,
        None,
    );

    ctx.repos.watch_event.save(&event).await?;

    let _ = ctx
        .services
        .event_publisher
        .publish(&DomainEvent::WatchEventIngested {
            user_id: event.user_id().clone(),
            title: event.title().to_string(),
            source: event.source().to_string(),
        })
        .await;

    Ok(())
}

#[cfg(test)]
#[path = "tests/ingest.rs"]
mod tests;
