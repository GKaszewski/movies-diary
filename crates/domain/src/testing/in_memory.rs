use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use chrono::NaiveDateTime;

use chrono::Utc;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        FederationFlags, Goal, ImportProfile, ImportSession, Movie, MovieFilter, MovieProfile,
        MovieSummary, ProfileField, RefreshSession, Review, User, UserSettings, UserSummary,
        WatchEvent, WatchEventStatus, WatchlistEntry, WatchlistWithMovie, WebhookToken,
        collections::{PageParams, Paginated},
    },
    ports::{
        GoalRepository, ImportProfileRepository, ImportSessionRepository, MovieProfileRepository,
        MovieRepository, RefreshSessionRepository, ReviewRepository, UserFederationSettingsQuery,
        UserProfileFieldsRepository, UserRepository, UserSettingsRepository, WatchEventRepository,
        WatchlistRepository, WebhookTokenRepository,
    },
    value_objects::{
        Email, ExternalMetadataId, GoalId, ImportProfileId, ImportSessionId, MovieId, MovieTitle,
        ReleaseYear, ReviewId, UserId, Username, WatchEventId, WebhookTokenId,
    },
};

// ── InMemoryMovieRepository ───────────────────────────────────────────────────

pub struct InMemoryMovieRepository {
    pub store: Mutex<HashMap<Uuid, Movie>>,
}

impl InMemoryMovieRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl MovieRepository for InMemoryMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|m| {
                m.external_metadata_id()
                    .map(|e| e.value() == external_metadata_id.value())
                    .unwrap_or(false)
            })
            .cloned())
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        Ok(self.store.lock().unwrap().get(&movie_id.value()).cloned())
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|m| m.title() == title && m.release_year() == year)
            .cloned()
            .collect())
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(movie.id().value(), movie.clone());
        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&movie_id.value());
        Ok(())
    }

    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        let store = self.store.lock().unwrap();
        let known: std::collections::HashSet<String> = store
            .values()
            .filter_map(|m| m.external_metadata_id().map(|e| e.value().to_string()))
            .collect();
        Ok(ids
            .iter()
            .map(|id| id.value().to_string())
            .filter(|v| known.contains(v))
            .collect())
    }

    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        let store = self.store.lock().unwrap();
        let known: std::collections::HashSet<(String, u16)> = store
            .values()
            .map(|m| (m.title().value().to_string(), m.release_year().value()))
            .collect();
        Ok(pairs
            .iter()
            .map(|(t, y)| (t.value().to_string(), y.value()))
            .filter(|p| known.contains(p))
            .collect())
    }

    async fn list_movies(
        &self,
        _page: &PageParams,
        _filter: &MovieFilter,
    ) -> Result<Paginated<MovieSummary>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn list_movies_with_external_id(&self) -> Result<Vec<Movie>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .values()
            .filter(|m| m.external_metadata_id().is_some())
            .cloned()
            .collect())
    }
}

// ── InMemoryReviewRepository ──────────────────────────────────────────────────

pub struct InMemoryReviewRepository {
    store: Mutex<HashMap<Uuid, Review>>,
}

impl InMemoryReviewRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl ReviewRepository for InMemoryReviewRepository {
    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(review.id().value(), review.clone());
        Ok(DomainEvent::ReviewLogged {
            review_id: review.id().clone(),
            movie_id: review.movie_id().clone(),
            user_id: review.user_id().clone(),
            rating: review.rating().clone(),
            watched_at: *review.watched_at(),
        })
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        Ok(self.store.lock().unwrap().get(&review_id.value()).cloned())
    }

    async fn update_review(&self, review: &Review) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(review.id().value(), review.clone());
        Ok(())
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&review_id.value());
        Ok(())
    }

    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|r| r.user_id() == user_id)
            .cloned()
            .collect())
    }
}

// ── InMemoryUserRepository ────────────────────────────────────────────────────

pub struct InMemoryUserRepository {
    pub store: Mutex<HashMap<Uuid, User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|u| u.email().value() == email.value())
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|u| u.username().value() == username.value())
            .cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(user.id().value(), user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        Ok(self.store.lock().unwrap().get(&id.value()).cloned())
    }

    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(vec![])
    }

    async fn update_profile(
        &self,
        _user_id: &UserId,
        _profile: &crate::models::UserProfile,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── InMemoryWatchlistRepository ───────────────────────────────────────────────

pub struct InMemoryWatchlistRepository {
    store: Mutex<HashMap<(Uuid, Uuid), WatchlistEntry>>,
}

impl InMemoryWatchlistRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl WatchlistRepository for InMemoryWatchlistRepository {
    async fn add(&self, entry: &WatchlistEntry) -> Result<(), DomainError> {
        let key = (entry.user_id.value(), entry.movie_id.value());
        self.store
            .lock()
            .unwrap()
            .entry(key)
            .or_insert_with(|| entry.clone());
        Ok(())
    }

    async fn remove(&self, user_id: &UserId, movie_id: &MovieId) -> Result<(), DomainError> {
        let key = (user_id.value(), movie_id.value());
        self.store
            .lock()
            .unwrap()
            .remove(&key)
            .ok_or_else(|| DomainError::NotFound("watchlist entry".into()))?;
        Ok(())
    }

    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
    ) -> Result<bool, DomainError> {
        let key = (user_id.value(), movie_id.value());
        Ok(self.store.lock().unwrap().remove(&key).is_some())
    }

    async fn get_for_user(
        &self,
        _user_id: &UserId,
        _page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn contains(&self, user_id: &UserId, movie_id: &MovieId) -> Result<bool, DomainError> {
        let key = (user_id.value(), movie_id.value());
        Ok(self.store.lock().unwrap().contains_key(&key))
    }
}

// ── InMemoryGoalRepository ──────────────────────────────────────────────────

pub struct InMemoryGoalRepository {
    store: Mutex<HashMap<Uuid, Goal>>,
    review_counts: Mutex<HashMap<(Uuid, u16), u32>>,
}

impl InMemoryGoalRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
            review_counts: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }

    pub fn set_review_count(&self, user_id: Uuid, year: u16, count: u32) {
        self.review_counts
            .lock()
            .unwrap()
            .insert((user_id, year), count);
    }
}

#[async_trait]
impl GoalRepository for InMemoryGoalRepository {
    async fn save(&self, goal: &Goal) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(goal.id().value(), goal.clone());
        Ok(())
    }

    async fn update(&self, goal: &Goal) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        match store.entry(goal.id().value()) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                e.insert(goal.clone());
                Ok(())
            }
            std::collections::hash_map::Entry::Vacant(_) => {
                Err(DomainError::NotFound("goal".into()))
            }
        }
    }

    async fn delete(&self, id: &GoalId, _user_id: &UserId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&id.value());
        Ok(())
    }

    async fn find_by_user_and_year(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<Goal>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|g| g.user_id().value() == user_id.value() && g.year() == year)
            .cloned())
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|g| g.user_id().value() == user_id.value())
            .cloned()
            .collect())
    }

    async fn count_reviews_in_year(&self, user_id: &UserId, year: u16) -> Result<u32, DomainError> {
        let counts = self.review_counts.lock().unwrap();
        Ok(counts.get(&(user_id.value(), year)).copied().unwrap_or(0))
    }
}

// ── InMemoryUserSettingsRepository ──────────────────────────────────────────

pub struct InMemoryUserSettingsRepository {
    store: Mutex<HashMap<Uuid, UserSettings>>,
}

impl InMemoryUserSettingsRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl UserSettingsRepository for InMemoryUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .get(&user_id.value())
            .cloned()
            .unwrap_or_else(|| UserSettings::new(user_id.clone())))
    }

    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(settings.user_id().value(), settings.clone());
        Ok(())
    }
}

#[async_trait]
impl UserFederationSettingsQuery for InMemoryUserSettingsRepository {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError> {
        let store = self.store.lock().unwrap();
        let settings = store
            .get(&user_id.value())
            .cloned()
            .unwrap_or_else(|| UserSettings::new(user_id.clone()));
        Ok(FederationFlags {
            goals: settings.federate_goals(),
            reviews: settings.federate_reviews(),
            watchlist: settings.federate_watchlist(),
        })
    }
}

// ── InMemoryWebhookTokenRepository ──────────────────────────────────────────

pub struct InMemoryWebhookTokenRepository {
    store: Mutex<Vec<WebhookToken>>,
}

impl InMemoryWebhookTokenRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Vec::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl WebhookTokenRepository for InMemoryWebhookTokenRepository {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(token.clone());
        Ok(())
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().find(|t| t.token_hash() == hash).cloned())
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|t| t.user_id().value() == user_id.value())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &WebhookTokenId, _user_id: &UserId) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .retain(|t| t.id().value() != id.value());
        Ok(())
    }

    async fn touch_last_used(&self, _id: &WebhookTokenId) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── InMemoryWatchEventRepository ────────────────────────────────────────────

pub struct InMemoryWatchEventRepository {
    store: Mutex<Vec<WatchEvent>>,
}

impl InMemoryWatchEventRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Vec::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl WatchEventRepository for InMemoryWatchEventRepository {
    async fn save(&self, event: &WatchEvent) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn update_status(
        &self,
        _id: &WatchEventId,
        _status: WatchEventStatus,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|e| {
                e.user_id().value() == user_id.value() && *e.status() == WatchEventStatus::Pending
            })
            .cloned()
            .collect())
    }

    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().find(|e| e.id().value() == id.value()).cloned())
    }

    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError> {
        let id_vals: Vec<Uuid> = ids.iter().map(|id| id.value()).collect();
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|e| id_vals.contains(&e.id().value()))
            .cloned()
            .collect())
    }

    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        _status: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        Ok(ids.len() as u64)
    }

    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: NaiveDateTime,
    ) -> Result<bool, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().any(|e| {
            e.user_id().value() == user_id.value()
                && e.external_metadata_id() == Some(external_id)
                && *e.watched_at() > after
        }))
    }

    async fn delete_non_pending_older_than(
        &self,
        before: NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let mut store = self.store.lock().unwrap();
        let before_len = store.len();
        store.retain(|e| *e.status() == WatchEventStatus::Pending || *e.created_at() >= before);
        Ok((before_len - store.len()) as u64)
    }
}

// ── InMemoryImportSessionRepository ─────────────────────────────────────────

pub struct InMemoryImportSessionRepository {
    store: Mutex<HashMap<Uuid, ImportSession>>,
}

impl InMemoryImportSessionRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl ImportSessionRepository for InMemoryImportSessionRepository {
    async fn create(&self, session: &ImportSession) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(session.id.value(), session.clone());
        Ok(())
    }

    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .get(&id.value())
            .filter(|s| s.user_id.value() == user_id.value())
            .cloned())
    }

    async fn update(&self, session: &ImportSession) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(session.id.value(), session.clone());
        Ok(())
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&id.value());
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let mut store = self.store.lock().unwrap();
        let now = chrono::Utc::now().naive_utc();
        let before_len = store.len();
        store.retain(|_, s| s.expires_at > now);
        Ok((before_len - store.len()) as u64)
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        let now = chrono::Utc::now().naive_utc();
        store.retain(|_, s| !(s.user_id.value() == user_id.value() && s.expires_at <= now));
        Ok(())
    }
}

// ── InMemoryImportProfileRepository ─────────────────────────────────────────

pub struct InMemoryImportProfileRepository {
    store: Mutex<HashMap<Uuid, ImportProfile>>,
}

impl InMemoryImportProfileRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl ImportProfileRepository for InMemoryImportProfileRepository {
    async fn save(&self, profile: &ImportProfile) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(profile.id.value(), profile.clone());
        Ok(())
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|p| p.user_id.value() == user_id.value())
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        id: &ImportProfileId,
        user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .get(&id.value())
            .filter(|p| p.user_id.value() == user_id.value())
            .cloned())
    }

    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&id.value());
        Ok(())
    }
}

// ── InMemoryMovieProfileRepository ──────────────────────────────────────────

pub struct InMemoryMovieProfileRepository {
    store: Mutex<HashMap<Uuid, MovieProfile>>,
}

impl InMemoryMovieProfileRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl MovieProfileRepository for InMemoryMovieProfileRepository {
    async fn upsert(&self, profile: &MovieProfile) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(profile.movie_id.value(), profile.clone());
        Ok(())
    }

    async fn get_by_movie_id(&self, id: &MovieId) -> Result<Option<MovieProfile>, DomainError> {
        Ok(self.store.lock().unwrap().get(&id.value()).cloned())
    }

    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError> {
        Ok(vec![])
    }
}

// ── InMemoryProfileFieldsRepo ───────────────────────────────────────────────

pub struct InMemoryProfileFieldsRepo {
    store: Mutex<HashMap<Uuid, Vec<ProfileField>>>,
}

impl InMemoryProfileFieldsRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl UserProfileFieldsRepository for InMemoryProfileFieldsRepo {
    async fn get_fields(&self, user_id: &UserId) -> Result<Vec<ProfileField>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.get(&user_id.value()).cloned().unwrap_or_default())
    }

    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<ProfileField>,
    ) -> Result<(), DomainError> {
        self.store.lock().unwrap().insert(user_id.value(), fields);
        Ok(())
    }
}

// ── InMemoryRefreshSessionRepository ────────────────────────────────────────

pub struct InMemoryRefreshSessionRepository {
    pub store: Mutex<Vec<RefreshSession>>,
}

impl InMemoryRefreshSessionRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl RefreshSessionRepository for InMemoryRefreshSessionRepository {
    async fn create(&self, session: &RefreshSession) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(session.clone());
        Ok(())
    }

    async fn get_by_token(&self, token: &str) -> Result<Option<RefreshSession>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.token == token)
            .cloned())
    }

    async fn revoke(&self, token: &str) -> Result<(), DomainError> {
        self.store.lock().unwrap().retain(|s| s.token != token);
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        self.store.lock().unwrap().retain(|s| s.user_id != *user_id);
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let mut store = self.store.lock().unwrap();
        let before = store.len();
        let now = Utc::now();
        store.retain(|s| s.expires_at >= now);
        Ok((before - store.len()) as u64)
    }
}
