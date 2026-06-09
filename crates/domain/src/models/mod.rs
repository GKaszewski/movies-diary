mod movie;
mod review;
mod user;
mod stats;
mod enrichment;
mod feed;

pub mod collections;
pub mod import;
pub mod import_profile;
pub mod import_session;
pub mod person;
pub mod search;
pub mod watchlist;
pub mod remote_watchlist;
pub mod goal;
pub mod user_settings;
pub mod remote_goal;
pub mod watch_event;
pub mod wrapup;

pub use movie::*;
pub use review::*;
pub use user::*;
pub use stats::*;
pub use enrichment::*;
pub use feed::*;

pub use watchlist::{WatchlistEntry, WatchlistWithMovie};
pub use remote_watchlist::RemoteWatchlistEntry;
pub use goal::{Goal, GoalWithProgress};
pub use user_settings::UserSettings;
pub use remote_goal::RemoteGoalEntry;
pub use watch_event::{
    ParsedPlaybackEvent, PersistedWatchEvent, WatchEvent, WatchEventSource, WatchEventStatus,
    WebhookToken,
};
pub use wrapup::*;

pub use import::{
    AnnotatedRow, DomainField, FieldMapping, FileFormat, ImportError, ImportRow, ParsedFile,
    RowResult, Transform,
};
pub use import_profile::ImportProfile;
pub use import_session::ImportSession;
pub use person::{CastCredit, CrewCredit, ExternalPersonId, Person, PersonCredits, PersonId};
pub use search::{
    EntityType, IndexableDocument, MovieSearchHit, PersonSearchHit, SearchFilters, SearchQuery,
    SearchResults,
};

use crate::errors::DomainError;

#[derive(Clone, Debug, PartialEq, Eq)]
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
pub enum SortDirection {
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
#[path = "tests.rs"]
mod tests;
