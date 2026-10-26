use domain::{errors::DomainError, models::ParsedPlaybackEvent, ports::MediaServerParser};
use serde::Deserialize;

pub struct JellyfinParser;

impl MediaServerParser for JellyfinParser {
    fn parse_playback_event(
        &self,
        body: &[u8],
    ) -> Result<Option<ParsedPlaybackEvent>, DomainError> {
        let payload: JellyfinPayload = serde_json::from_slice(body)
            .map_err(|e| DomainError::ValidationError(format!("invalid Jellyfin payload: {e}")))?;

        if payload.notification_type != "PlaybackStop" {
            return Ok(None);
        }

        let item_type = payload.item_type.as_deref().unwrap_or("");
        if item_type != "Movie" {
            return Ok(None);
        }

        if !payload.played_to_completion.unwrap_or(false) {
            return Ok(None);
        }

        let title = match payload.name {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(None),
        };

        let tmdb_id = payload.provider_tmdb.map(|id| format!("tmdb:{id}"));
        let imdb_id = payload.provider_imdb;

        Ok(Some(ParsedPlaybackEvent {
            title,
            year: payload.year,
            tmdb_id,
            imdb_id,
        }))
    }
}

#[derive(Deserialize)]
struct JellyfinPayload {
    #[serde(rename = "NotificationType")]
    notification_type: String,
    #[serde(rename = "ItemType")]
    item_type: Option<String>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Year")]
    year: Option<u16>,
    #[serde(rename = "PlayedToCompletion")]
    played_to_completion: Option<bool>,
    #[serde(rename = "Provider_tmdb")]
    provider_tmdb: Option<String>,
    #[serde(rename = "Provider_imdb")]
    provider_imdb: Option<String>,
}
