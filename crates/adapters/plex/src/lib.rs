use domain::{errors::DomainError, models::ParsedPlaybackEvent, ports::MediaServerParser};
use serde::Deserialize;

pub struct PlexParser;

impl MediaServerParser for PlexParser {
    /// Plex sends multipart form data with a `payload` JSON field.
    /// The caller must extract the JSON string from the multipart body
    /// and pass it here as raw bytes.
    fn parse_playback_event(
        &self,
        body: &[u8],
    ) -> Result<Option<ParsedPlaybackEvent>, DomainError> {
        let payload: PlexPayload = serde_json::from_slice(body)
            .map_err(|e| DomainError::ValidationError(format!("invalid Plex payload: {e}")))?;

        if payload.event != "media.scrobble" {
            return Ok(None);
        }

        let metadata = match payload.metadata {
            Some(m) => m,
            None => return Ok(None),
        };

        if metadata.media_type != "movie" {
            return Ok(None);
        }

        if metadata.title.is_empty() {
            return Ok(None);
        }

        let mut tmdb_id = None;
        let mut imdb_id = None;
        for guid in &metadata.guids {
            if let Some(id) = guid.id.strip_prefix("tmdb://") {
                tmdb_id = Some(format!("tmdb:{id}"));
            } else if let Some(id) = guid.id.strip_prefix("imdb://") {
                imdb_id = Some(id.to_string());
            }
        }

        Ok(Some(ParsedPlaybackEvent {
            title: metadata.title,
            year: metadata.year.map(|y| y as u16),
            tmdb_id,
            imdb_id,
        }))
    }
}

#[derive(Deserialize)]
struct PlexPayload {
    event: String,
    #[serde(rename = "Metadata")]
    metadata: Option<PlexMetadata>,
}

#[derive(Deserialize)]
struct PlexMetadata {
    #[serde(rename = "type")]
    media_type: String,
    title: String,
    year: Option<i32>,
    #[serde(rename = "Guid", default)]
    guids: Vec<PlexGuid>,
}

#[derive(Deserialize)]
struct PlexGuid {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_movie_scrobble() {
        let body = serde_json::json!({
            "event": "media.scrobble",
            "Metadata": {
                "type": "movie",
                "title": "Blade Runner",
                "year": 1982,
                "Guid": [
                    {"id": "tmdb://78"},
                    {"id": "imdb://tt0083658"}
                ]
            }
        });
        let parser = PlexParser;
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
    fn ignores_tv_episode() {
        let body = serde_json::json!({
            "event": "media.scrobble",
            "Metadata": {
                "type": "episode",
                "title": "Pilot",
                "grandparentTitle": "Breaking Bad",
                "year": 2008,
                "Guid": []
            }
        });
        let parser = PlexParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn ignores_play_event() {
        let body = serde_json::json!({
            "event": "media.play",
            "Metadata": {
                "type": "movie",
                "title": "Blade Runner",
                "year": 1982,
                "Guid": []
            }
        });
        let parser = PlexParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn handles_no_guids() {
        let body = serde_json::json!({
            "event": "media.scrobble",
            "Metadata": {
                "type": "movie",
                "title": "Some Indie Film",
                "year": 2023
            }
        });
        let parser = PlexParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        let event = result.expect("should parse");
        assert_eq!(event.title, "Some Indie Film");
        assert!(event.tmdb_id.is_none());
        assert!(event.imdb_id.is_none());
    }

    #[test]
    fn handles_missing_metadata() {
        let body = serde_json::json!({
            "event": "media.scrobble"
        });
        let parser = PlexParser;
        let result = parser
            .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
            .unwrap();
        assert!(result.is_none());
    }
}
