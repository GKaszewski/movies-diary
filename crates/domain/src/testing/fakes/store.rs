use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::models::{
    Goal, ImportProfile, ImportSession, Movie, MovieProfile, ProfileField, RefreshSession,
    RemoteWatchlistEntry, Review, User, UserSettings, WatchEvent, WatchlistEntry, WebhookToken,
};
use crate::value_objects::SocialIdentity;

pub(super) struct LedgerRow {
    pub(super) done: bool,
    pub(super) started_at: chrono::NaiveDateTime,
    pub(super) processed_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FollowState {
    Pending,
    Accepted,
}

pub const USER_CASCADE_TABLES: &[&str] = &[
    "watchlist_entries",
    "goals",
    "user_settings",
    "user_profile_fields",
    "webhook_tokens",
    "watch_events",
    "ap_followers",
    "ap_following",
    "blocked_actors",
];

pub const USER_CASCADE_TABLES_OUTSIDE_STORE: &[&str] = &["ap_local_actors"];

pub const MOVIE_CASCADE_TABLES: &[&str] = &[
    "watchlist_entries",
    "movie_profiles",
    "movie_genres",
    "movie_keywords",
    "movie_cast",
    "movie_crew",
];

pub struct InMemoryStore {
    pub(super) movies: RwLock<HashMap<Uuid, Movie>>,
    pub(super) movie_profiles: RwLock<HashMap<Uuid, MovieProfile>>,
    pub(super) reviews: RwLock<HashMap<Uuid, Review>>,
    pub(super) users: RwLock<HashMap<Uuid, User>>,
    pub(super) tombstoned: RwLock<std::collections::HashSet<Uuid>>,
    pub(super) retired_usernames: RwLock<std::collections::HashSet<String>>,
    pub(super) profile_fields: RwLock<HashMap<Uuid, Vec<ProfileField>>>,
    pub(super) user_settings: RwLock<HashMap<Uuid, UserSettings>>,
    pub(super) refresh_sessions: RwLock<Vec<RefreshSession>>,
    pub(super) watchlist: RwLock<HashMap<(Uuid, Uuid), WatchlistEntry>>,
    pub(super) remote_watchlist: RwLock<Vec<RemoteWatchlistEntry>>,
    pub(super) goals: RwLock<HashMap<Uuid, Goal>>,
    pub(super) watch_events: RwLock<Vec<WatchEvent>>,
    pub(super) webhook_tokens: RwLock<Vec<WebhookToken>>,
    pub(super) import_sessions: RwLock<HashMap<Uuid, ImportSession>>,
    pub(super) import_profiles: RwLock<HashMap<Uuid, ImportProfile>>,
    pub(super) follows: RwLock<Vec<(Uuid, SocialIdentity, FollowState)>>,
    pub(super) blocked: RwLock<Vec<(Uuid, SocialIdentity)>>,
    pub(super) processed_events: RwLock<HashMap<(Uuid, String), LedgerRow>>,
    pub(super) recorded_events: RwLock<Vec<crate::events::DomainEvent>>,
}

impl InMemoryStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            movies: RwLock::default(),
            movie_profiles: RwLock::default(),
            reviews: RwLock::default(),
            users: RwLock::default(),
            tombstoned: RwLock::default(),
            retired_usernames: RwLock::default(),
            profile_fields: RwLock::default(),
            user_settings: RwLock::default(),
            refresh_sessions: RwLock::default(),
            watchlist: RwLock::default(),
            remote_watchlist: RwLock::default(),
            goals: RwLock::default(),
            watch_events: RwLock::default(),
            webhook_tokens: RwLock::default(),
            import_sessions: RwLock::default(),
            import_profiles: RwLock::default(),
            follows: RwLock::default(),
            blocked: RwLock::default(),
            processed_events: RwLock::default(),
            recorded_events: RwLock::default(),
        })
    }

    pub fn with_remote_watchlist(entries: Vec<RemoteWatchlistEntry>) -> Arc<Self> {
        let store = Self::new();
        *store.remote_watchlist.write().unwrap() = entries;
        store
    }

    pub fn recorded_events(&self) -> Vec<crate::events::DomainEvent> {
        self.recorded_events.read().unwrap().clone()
    }

    pub(super) fn record_events(&self, events: &[crate::events::DomainEvent]) {
        self.recorded_events
            .write()
            .unwrap()
            .extend_from_slice(events);
    }

    pub fn movie_count(&self) -> usize {
        self.movies.read().unwrap().len()
    }

    pub fn movie_profile_count(&self) -> usize {
        self.movie_profiles.read().unwrap().len()
    }

    pub fn review_count(&self) -> usize {
        self.reviews.read().unwrap().len()
    }

    pub fn user_count(&self) -> usize {
        self.users.read().unwrap().len()
    }

    pub fn watchlist_count(&self) -> usize {
        self.watchlist.read().unwrap().len()
    }

    pub fn goal_count(&self) -> usize {
        self.goals.read().unwrap().len()
    }

    pub fn import_session_count(&self) -> usize {
        self.import_sessions.read().unwrap().len()
    }

    pub fn processed_event_count(&self) -> usize {
        self.processed_events.read().unwrap().len()
    }

    pub fn seed_processed_event(
        &self,
        id: &crate::events::EventId,
        handler: &str,
        done: bool,
        at: chrono::NaiveDateTime,
    ) {
        self.processed_events.write().unwrap().insert(
            (id.value(), handler.to_owned()),
            LedgerRow {
                done,
                started_at: at,
                processed_at: at,
            },
        );
    }

    pub fn refresh_session_count(&self) -> usize {
        self.refresh_sessions.read().unwrap().len()
    }

    pub fn seed_user(&self, user: User) {
        self.users.write().unwrap().insert(user.id().value(), user);
    }

    pub fn rows_in(&self, table: &str) -> usize {
        match table {
            "watchlist_entries" => self.watchlist_count(),
            "goals" => self.goal_count(),
            "user_settings" => self.user_settings.read().unwrap().len(),
            "user_profile_fields" => self
                .profile_fields
                .read()
                .unwrap()
                .values()
                .map(Vec::len)
                .sum(),
            "webhook_tokens" => self.webhook_tokens.read().unwrap().len(),
            "watch_events" => self.watch_events.read().unwrap().len(),
            "ap_following" => self.follows.read().unwrap().len(),
            "ap_followers" => self
                .follows
                .read()
                .unwrap()
                .iter()
                .filter(|(_, target, _)| matches!(target, SocialIdentity::Local(_)))
                .count(),
            "blocked_actors" => self.blocked.read().unwrap().len(),
            "movie_profiles" => self.movie_profile_count(),
            "movie_genres" => self.sum_profiles(|p| p.genres.len()),
            "movie_keywords" => self.sum_profiles(|p| p.keywords.len()),
            "movie_cast" => self.sum_profiles(|p| p.cast.len()),
            "movie_crew" => self.sum_profiles(|p| p.crew.len()),
            other => panic!("InMemoryStore does not model table {other}"),
        }
    }

    fn sum_profiles(&self, f: impl Fn(&MovieProfile) -> usize) -> usize {
        self.movie_profiles.read().unwrap().values().map(f).sum()
    }
}
