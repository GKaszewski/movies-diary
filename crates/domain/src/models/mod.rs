mod enrichment;
mod federation;
mod feed;
mod movie;
mod refresh_session;
mod review;
mod stats;
mod user;

pub mod collections;
pub mod goal;
pub mod import;
pub mod import_profile;
pub mod import_session;
pub mod person;
pub mod remote_goal;
pub mod remote_watchlist;
pub mod search;
pub mod user_settings;
pub mod watch_event;
pub mod watchlist;
pub mod wrapup;

pub use enrichment::*;
pub use federation::*;
pub use feed::*;
pub use movie::*;
pub use review::*;
pub use stats::*;
pub use user::*;

pub use goal::{Goal, GoalWithProgress};
pub use remote_goal::RemoteGoalEntry;
pub use remote_watchlist::RemoteWatchlistEntry;
pub use user_settings::UserSettings;
pub use watch_event::{
    ParsedPlaybackEvent, PersistedWatchEvent, WatchEvent, WatchEventSource, WatchEventStatus,
    WebhookToken,
};
pub use watchlist::{WatchlistEntry, WatchlistWithMovie};
pub use wrapup::*;

pub use import::{
    AnnotatedRow, DomainField, FieldMapping, FileFormat, ImportError, ImportRow, ParsedFile,
    RowResult, Transform,
};
pub use import_profile::ImportProfile;
pub use import_session::ImportSession;
pub use person::{
    CastCredit, CrewCredit, ExternalPersonId, Person, PersonCredits, PersonEnrichmentData,
};
pub use crate::value_objects::PersonId;
pub use refresh_session::{GeneratedToken, RefreshSession};
pub use search::{
    EntityType, IndexableDocument, MovieSearchHit, PersonSearchHit, SearchFilters, SearchQuery,
    SearchResults,
};

use crate::errors::DomainError;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
    Movies,
}

impl GoalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Movies => "movies",
        }
    }
}

impl std::str::FromStr for GoalType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "movies" => Ok(Self::Movies),
            other => Err(DomainError::ValidationError(format!(
                "Unknown goal type: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum ReviewSortBy {
    #[default]
    Descending,
    Ascending,
    ByRatingDesc,
    ByRatingAsc,
}

pub enum ExportFormat {
    Csv,
    Json,
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
