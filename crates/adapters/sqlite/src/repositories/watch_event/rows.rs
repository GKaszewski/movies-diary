use domain::{
    errors::DomainError,
    models::{PersistedWatchEvent, WatchEvent, WatchEventSource, WatchEventStatus, WebhookToken},
    value_objects::{MovieId, UserId, WatchEventId, WebhookTokenId},
};
use sqlx::Row;

pub(super) fn parse_uuid(s: &str) -> Result<uuid::Uuid, DomainError> {
    s.parse()
        .map_err(|_| DomainError::InfrastructureError(format!("invalid UUID: {s}")))
}

pub(super) fn parse_datetime(s: &str) -> Result<chrono::NaiveDateTime, DomainError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| DomainError::InfrastructureError(format!("invalid datetime: {s}")))
}

pub(super) fn row_to_watch_event(row: &sqlx::sqlite::SqliteRow) -> Result<WatchEvent, DomainError> {
    let id_str: &str = row.try_get("id").map_err(crate::shared::map_sqlx_error)?;
    let user_id_str: &str = row
        .try_get("user_id")
        .map_err(crate::shared::map_sqlx_error)?;
    let movie_id_str: Option<&str> = row
        .try_get("movie_id")
        .map_err(crate::shared::map_sqlx_error)?;
    let title: String = row
        .try_get("title")
        .map_err(crate::shared::map_sqlx_error)?;
    let year: Option<i64> = row.try_get("year").map_err(crate::shared::map_sqlx_error)?;
    let ext_id: Option<String> = row
        .try_get("external_metadata_id")
        .map_err(crate::shared::map_sqlx_error)?;
    let source_str: String = row
        .try_get("source")
        .map_err(crate::shared::map_sqlx_error)?;
    let watched_at_str: String = row
        .try_get("watched_at")
        .map_err(crate::shared::map_sqlx_error)?;
    let status_str: String = row
        .try_get("status")
        .map_err(crate::shared::map_sqlx_error)?;
    let created_at_str: String = row
        .try_get("created_at")
        .map_err(crate::shared::map_sqlx_error)?;

    let source: WatchEventSource = source_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;
    let status: WatchEventStatus = status_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let movie_id = movie_id_str
        .map(parse_uuid)
        .transpose()?
        .map(MovieId::from_uuid);

    Ok(WatchEvent::from_persistence(PersistedWatchEvent {
        id: WatchEventId::from_uuid(parse_uuid(id_str)?),
        user_id: UserId::from_uuid(parse_uuid(user_id_str)?),
        movie_id,
        title,
        year: year.map(|y| y as u16),
        external_metadata_id: ext_id,
        source,
        watched_at: parse_datetime(&watched_at_str)?,
        status,
        created_at: parse_datetime(&created_at_str)?,
    }))
}

pub(super) fn row_to_webhook_token(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<WebhookToken, DomainError> {
    let id_str: &str = row.try_get("id").map_err(crate::shared::map_sqlx_error)?;
    let user_id_str: &str = row
        .try_get("user_id")
        .map_err(crate::shared::map_sqlx_error)?;
    let token_hash: String = row
        .try_get("token_hash")
        .map_err(crate::shared::map_sqlx_error)?;
    let provider_str: String = row
        .try_get("provider")
        .map_err(crate::shared::map_sqlx_error)?;
    let label: Option<String> = row
        .try_get("label")
        .map_err(crate::shared::map_sqlx_error)?;
    let created_at_str: String = row
        .try_get("created_at")
        .map_err(crate::shared::map_sqlx_error)?;
    let last_used_str: Option<String> = row
        .try_get("last_used_at")
        .map_err(crate::shared::map_sqlx_error)?;

    let provider: WatchEventSource = provider_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let last_used = last_used_str.map(|s| parse_datetime(&s)).transpose()?;

    Ok(WebhookToken::from_persistence(
        WebhookTokenId::from_uuid(parse_uuid(id_str)?),
        UserId::from_uuid(parse_uuid(user_id_str)?),
        token_hash,
        provider,
        label,
        parse_datetime(&created_at_str)?,
        last_used,
    ))
}
