use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        Movie, MovieFilter, MovieSummary, Review, User, UserSummary, WatchlistEntry,
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::{MovieRepository, ReviewRepository, UserRepository, WatchlistRepository},
    value_objects::{
        Email, ExternalMetadataId, MovieId, MovieTitle, ReleaseYear, ReviewId, UserId, Username,
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
