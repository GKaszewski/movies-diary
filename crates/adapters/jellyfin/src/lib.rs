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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_playback_stop() {
        let body = serde_json::json!({
            "NotificationType": "PlaybackStop",
            "ItemType": "Movie",
            "Name": "Blade Runner",
            "Year": 1982,
            "PlayedToCompletion": true,
            "Provider_tmdb": "78",
            "Provider_imdb": "tt0083658"
        });
        let parser = JellyfinParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        let event = result.expect("should parse");
        assert_eq!(event.title, "Blade Runner");
        assert_eq!(event.year, Some(1982));
        assert_eq!(event.tmdb_id, Some("tmdb:78".into()));
        assert_eq!(event.imdb_id, Some("tt0083658".into()));
    }

    #[test]
    fn ignores_non_movie() {
        let body = serde_json::json!({
            "NotificationType": "PlaybackStop",
            "ItemType": "Episode",
            "Name": "Some Episode",
            "PlayedToCompletion": true
        });
        let parser = JellyfinParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn ignores_incomplete_playback() {
        let body = serde_json::json!({
            "NotificationType": "PlaybackStop",
            "ItemType": "Movie",
            "Name": "Blade Runner",
            "PlayedToCompletion": false
        });
        let parser = JellyfinParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn ignores_playback_start() {
        let body = serde_json::json!({
            "NotificationType": "PlaybackStart",
            "ItemType": "Movie",
            "Name": "Blade Runner",
            "PlayedToCompletion": false
        });
        let parser = JellyfinParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }
}
