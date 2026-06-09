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
mod tests {
    use super::*;

    fn ts() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    #[test]
    fn watch_event_new_has_pending_status() {
        let e = WatchEvent::new(
            UserId::generate(),
            "Dune".into(),
            Some(2021),
            None,
            WatchEventSource::Jellyfin,
            ts(),
            None,
        );
        assert_eq!(*e.status(), WatchEventStatus::Pending);
    }

    #[test]
    fn watch_event_getters() {
        let uid = UserId::generate();
        let mid = MovieId::generate();
        let e = WatchEvent::new(
            uid.clone(),
            "Arrival".into(),
            Some(2016),
            Some("ext123".into()),
            WatchEventSource::Plex,
            ts(),
            Some(mid.clone()),
        );
        assert_eq!(*e.user_id(), uid);
        assert_eq!(e.title(), "Arrival");
        assert_eq!(e.year(), Some(2016));
        assert_eq!(e.external_metadata_id(), Some("ext123"));
        assert_eq!(*e.source(), WatchEventSource::Plex);
        assert_eq!(e.watched_at(), &ts());
        assert_eq!(*e.movie_id().unwrap(), mid);
    }

    #[test]
    fn webhook_token_new() {
        let uid = UserId::generate();
        let t = WebhookToken::new(
            uid.clone(),
            "hash123".into(),
            WatchEventSource::Jellyfin,
            Some("my server".into()),
        );
        assert_eq!(*t.user_id(), uid);
        assert_eq!(t.token_hash(), "hash123");
        assert_eq!(*t.provider(), WatchEventSource::Jellyfin);
        assert_eq!(t.label(), Some("my server"));
        assert!(t.last_used_at().is_none());
    }

    #[test]
    fn webhook_token_from_persistence() {
        let id = WebhookTokenId::generate();
        let uid = UserId::generate();
        let created = ts();
        let used = chrono::NaiveDate::from_ymd_opt(2024, 7, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let t = WebhookToken::from_persistence(
            id.clone(),
            uid.clone(),
            "h".into(),
            WatchEventSource::Plex,
            None,
            created,
            Some(used),
        );
        assert_eq!(*t.id(), id);
        assert_eq!(*t.user_id(), uid);
        assert_eq!(t.token_hash(), "h");
        assert_eq!(*t.provider(), WatchEventSource::Plex);
        assert_eq!(t.label(), None);
        assert_eq!(t.created_at(), &created);
        assert_eq!(t.last_used_at(), Some(&used));
    }

    #[test]
    fn watch_event_source_display() {
        assert_eq!(WatchEventSource::Jellyfin.to_string(), "jellyfin");
        assert_eq!(WatchEventSource::Plex.to_string(), "plex");
    }

    #[test]
    fn watch_event_source_from_str() {
        assert_eq!(
            "jellyfin".parse::<WatchEventSource>().unwrap(),
            WatchEventSource::Jellyfin
        );
        assert_eq!(
            "plex".parse::<WatchEventSource>().unwrap(),
            WatchEventSource::Plex
        );
        assert!("unknown".parse::<WatchEventSource>().is_err());
    }

    #[test]
    fn watch_event_status_display() {
        assert_eq!(WatchEventStatus::Pending.to_string(), "pending");
        assert_eq!(WatchEventStatus::Confirmed.to_string(), "confirmed");
        assert_eq!(WatchEventStatus::Dismissed.to_string(), "dismissed");
    }

    #[test]
    fn watch_event_status_from_str() {
        for s in ["pending", "confirmed", "dismissed"] {
            let parsed: WatchEventStatus = s.parse().unwrap();
            assert_eq!(parsed.to_string(), s);
        }
        assert!("bogus".parse::<WatchEventStatus>().is_err());
    }

    #[test]
    fn parsed_playback_event_fields() {
        let p = ParsedPlaybackEvent {
            title: "Matrix".into(),
            year: Some(1999),
            tmdb_id: Some("603".into()),
            imdb_id: Some("tt0133093".into()),
        };
        assert_eq!(p.title, "Matrix");
        assert_eq!(p.year, Some(1999));
        assert_eq!(p.tmdb_id.as_deref(), Some("603"));
        assert_eq!(p.imdb_id.as_deref(), Some("tt0133093"));
    }
}
