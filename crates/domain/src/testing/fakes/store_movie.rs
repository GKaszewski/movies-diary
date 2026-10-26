use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        Movie, MovieFilter, MovieProfile, MovieSummary, PersistedWatchEvent, WatchEvent,
        collections::{PageParams, Paginated},
    },
    ports::{MovieCommand, MovieProfileRepository, MovieQuery},
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear},
};

use super::InMemoryStore;

#[async_trait]
impl MovieCommand for InMemoryStore {
    async fn upsert_movie(&self, movie: &Movie, events: &[DomainEvent]) -> Result<(), DomainError> {
        let mut movies = self.movies.write().unwrap();
        if let Some(external) = movie.external_metadata_id() {
            let taken = movies.values().any(|m| {
                m.id() != movie.id()
                    && m.external_metadata_id().map(|e| e.value()) == Some(external.value())
            });
            if taken {
                return Err(DomainError::InfrastructureError(
                    "Database operation failed".into(),
                ));
            }
        }
        movies.insert(movie.id().value(), movie.clone());
        drop(movies);
        self.record_events(events);
        Ok(())
    }

    async fn delete_movie(
        &self,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id = movie_id.value();
        if !self.movies.read().unwrap().contains_key(&id) {
            return Ok(());
        }
        if self
            .reviews
            .read()
            .unwrap()
            .values()
            .any(|r| r.movie_id().value() == id)
        {
            return Err(DomainError::InfrastructureError(
                "Database operation failed".into(),
            ));
        }
        self.remove_movie_with_cascades(id);
        self.record_events(events);
        Ok(())
    }
}

impl InMemoryStore {
    pub(super) fn remove_movie_with_cascades(&self, id: uuid::Uuid) {
        self.movies.write().unwrap().remove(&id);
        self.watchlist
            .write()
            .unwrap()
            .retain(|(_, movie), _| *movie != id);
        self.movie_profiles.write().unwrap().remove(&id);
        let mut events_guard = self.watch_events.write().unwrap();
        for event in events_guard.iter_mut() {
            if event.movie_id().map(|m| m.value()) == Some(id) {
                *event = detached_from_movie(event);
            }
        }
    }
}

#[async_trait]
impl MovieQuery for InMemoryStore {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let store = self.movies.read().unwrap();
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
        Ok(self.movies.read().unwrap().get(&movie_id.value()).cloned())
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let store = self.movies.read().unwrap();
        Ok(store
            .values()
            .filter(|m| m.title() == title && m.release_year() == year)
            .cloned()
            .collect())
    }

    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        let store = self.movies.read().unwrap();
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
        let store = self.movies.read().unwrap();
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
        page: &PageParams,
        filter: &MovieFilter,
    ) -> Result<Paginated<MovieSummary>, DomainError> {
        let movies = self.movies.read().unwrap();
        let profiles = self.movie_profiles.read().unwrap();
        let search = filter.search.as_deref().map(str::to_lowercase);
        let mut rows: Vec<MovieSummary> = movies
            .values()
            .filter_map(|m| {
                let profile = profiles.get(&m.id().value());
                let genres: Vec<String> = profile
                    .map(|p| p.genres.iter().map(|g| g.name.clone()).collect())
                    .unwrap_or_default();
                let language = profile.and_then(|p| p.original_language.clone());
                let keep = search
                    .as_ref()
                    .is_none_or(|q| m.title().value().to_lowercase().contains(q))
                    && filter
                        .language
                        .as_ref()
                        .is_none_or(|l| language.as_ref() == Some(l))
                    && filter.genre.as_ref().is_none_or(|g| {
                        genres.iter().any(|n| n.to_lowercase() == g.to_lowercase())
                    });
                keep.then(|| MovieSummary {
                    movie: m.clone(),
                    genres,
                    runtime_minutes: profile.and_then(|p| p.runtime_minutes),
                    original_language: language,
                    overview: profile.and_then(|p| p.overview.clone()),
                    collection_name: profile.and_then(|p| p.collection_name.clone()),
                })
            })
            .collect();
        rows.sort_by(|a, b| a.movie.title().value().cmp(b.movie.title().value()));
        let total_count = rows.len() as u64;
        let items = rows
            .into_iter()
            .skip(page.offset as usize)
            .take(page.limit as usize)
            .collect();
        Ok(Paginated {
            items,
            total_count,
            limit: page.limit,
            offset: page.offset,
        })
    }

    async fn list_movies_with_external_id(&self) -> Result<Vec<Movie>, DomainError> {
        Ok(self
            .movies
            .read()
            .unwrap()
            .values()
            .filter(|m| m.external_metadata_id().is_some())
            .cloned()
            .collect())
    }
}

#[async_trait]
impl MovieProfileRepository for InMemoryStore {
    async fn upsert(&self, profile: &MovieProfile) -> Result<(), DomainError> {
        self.movie_profiles
            .write()
            .unwrap()
            .insert(profile.movie_id.value(), profile.clone());
        Ok(())
    }

    async fn get_by_movie_id(&self, id: &MovieId) -> Result<Option<MovieProfile>, DomainError> {
        Ok(self
            .movie_profiles
            .read()
            .unwrap()
            .get(&id.value())
            .cloned())
    }

    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError> {
        Ok(vec![])
    }
}

fn detached_from_movie(event: &WatchEvent) -> WatchEvent {
    WatchEvent::from_persistence(PersistedWatchEvent {
        id: event.id().clone(),
        user_id: event.user_id().clone(),
        movie_id: None,
        title: event.title().to_string(),
        year: event.year(),
        external_metadata_id: event.external_metadata_id().map(str::to_string),
        source: event.source().clone(),
        watched_at: *event.watched_at(),
        status: event.status().clone(),
        created_at: *event.created_at(),
    })
}
