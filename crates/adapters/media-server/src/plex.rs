use domain::{errors::DomainError, models::ParsedPlaybackEvent, ports::MediaServerParser};
use serde::Deserialize;

pub struct PlexParser;

impl MediaServerParser for PlexParser {
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
