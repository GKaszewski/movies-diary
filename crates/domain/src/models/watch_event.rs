use chrono::NaiveDateTime;

use crate::value_objects::{MovieId, UserId, WatchEventId, WebhookTokenId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchEventSource {
    Jellyfin,
    Plex,
}

impl std::fmt::Display for WatchEventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Jellyfin => write!(f, "jellyfin"),
            Self::Plex => write!(f, "plex"),
        }
    }
}

impl std::str::FromStr for WatchEventSource {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jellyfin" => Ok(Self::Jellyfin),
            "plex" => Ok(Self::Plex),
            other => Err(format!("unknown watch event source: {other}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum WatchEventStatus {
    #[default]
    Pending,
    Confirmed,
    Dismissed,
}

impl std::fmt::Display for WatchEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Confirmed => write!(f, "confirmed"),
            Self::Dismissed => write!(f, "dismissed"),
        }
    }
}

impl std::str::FromStr for WatchEventStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "confirmed" => Ok(Self::Confirmed),
            "dismissed" => Ok(Self::Dismissed),
            other => Err(format!("unknown watch event status: {other}")),
        }
    }
}

pub struct PersistedWatchEvent {
    pub id: WatchEventId,
    pub user_id: UserId,
    pub movie_id: Option<MovieId>,
    pub title: String,
    pub year: Option<u16>,
    pub external_metadata_id: Option<String>,
    pub source: WatchEventSource,
    pub watched_at: NaiveDateTime,
    pub status: WatchEventStatus,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug)]
pub struct WatchEvent {
    id: WatchEventId,
    user_id: UserId,
    movie_id: Option<MovieId>,
    title: String,
    year: Option<u16>,
    external_metadata_id: Option<String>,
    source: WatchEventSource,
    watched_at: NaiveDateTime,
    status: WatchEventStatus,
    created_at: NaiveDateTime,
}

impl WatchEvent {
    pub fn new(
        user_id: UserId,
        title: String,
        year: Option<u16>,
        external_metadata_id: Option<String>,
        source: WatchEventSource,
        watched_at: NaiveDateTime,
        movie_id: Option<MovieId>,
    ) -> Self {
        Self {
            id: WatchEventId::generate(),
            user_id,
            movie_id,
            title,
            year,
            external_metadata_id,
            source,
            watched_at,
            status: WatchEventStatus::Pending,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_persistence(row: PersistedWatchEvent) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            movie_id: row.movie_id,
            title: row.title,
            year: row.year,
            external_metadata_id: row.external_metadata_id,
            source: row.source,
            watched_at: row.watched_at,
            status: row.status,
            created_at: row.created_at,
        }
    }

    pub fn id(&self) -> &WatchEventId {
        &self.id
    }
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    pub fn movie_id(&self) -> Option<&MovieId> {
        self.movie_id.as_ref()
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn year(&self) -> Option<u16> {
        self.year
    }
    pub fn external_metadata_id(&self) -> Option<&str> {
        self.external_metadata_id.as_deref()
    }
    pub fn source(&self) -> &WatchEventSource {
        &self.source
    }
    pub fn watched_at(&self) -> &NaiveDateTime {
        &self.watched_at
    }
    pub fn status(&self) -> &WatchEventStatus {
        &self.status
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct WebhookToken {
    id: WebhookTokenId,
    user_id: UserId,
    token_hash: String,
    provider: WatchEventSource,
    label: Option<String>,
    created_at: NaiveDateTime,
    last_used_at: Option<NaiveDateTime>,
}

impl WebhookToken {
    pub fn new(
        user_id: UserId,
        token_hash: String,
        provider: WatchEventSource,
        label: Option<String>,
    ) -> Self {
        Self {
            id: WebhookTokenId::generate(),
            user_id,
            token_hash,
            provider,
            label,
            created_at: chrono::Utc::now().naive_utc(),
            last_used_at: None,
        }
    }

    pub fn from_persistence(
        id: WebhookTokenId,
        user_id: UserId,
        token_hash: String,
        provider: WatchEventSource,
        label: Option<String>,
        created_at: NaiveDateTime,
        last_used_at: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id,
            user_id,
            token_hash,
            provider,
            label,
            created_at,
            last_used_at,
        }
    }

    pub fn id(&self) -> &WebhookTokenId {
        &self.id
    }
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn provider(&self) -> &WatchEventSource {
        &self.provider
    }
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.created_at
    }
    pub fn last_used_at(&self) -> Option<&NaiveDateTime> {
        self.last_used_at.as_ref()
    }
}

pub struct ParsedPlaybackEvent {
    pub title: String,
    pub year: Option<u16>,
    pub tmdb_id: Option<String>,
    pub imdb_id: Option<String>,
}

#[cfg(test)]
#[path = "tests/watch_event.rs"]
mod tests;
