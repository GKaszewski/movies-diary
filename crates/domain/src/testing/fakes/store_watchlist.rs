use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        RemoteWatchlistEntry, WatchlistEntry, WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::{RemoteWatchlistRepository, WatchlistRepository},
    value_objects::{MovieId, UserId},
};

use super::InMemoryStore;

#[async_trait]
impl WatchlistRepository for InMemoryStore {
    async fn add(&self, entry: &WatchlistEntry, events: &[DomainEvent]) -> Result<(), DomainError> {
        let key = (entry.user_id.value(), entry.movie_id.value());
        let inserted = {
            let mut entries = self.watchlist.write().unwrap();
            match entries.entry(key) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(entry.clone());
                    true
                }
                std::collections::hash_map::Entry::Occupied(_) => false,
            }
        };
        if inserted {
            self.record_events(events);
        }
        Ok(())
    }

    async fn remove(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let key = (user_id.value(), movie_id.value());
        self.watchlist
            .write()
            .unwrap()
            .remove(&key)
            .ok_or_else(|| DomainError::NotFound("watchlist entry".into()))?;
        self.record_events(events);
        Ok(())
    }

    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<bool, DomainError> {
        let key = (user_id.value(), movie_id.value());
        let removed = self.watchlist.write().unwrap().remove(&key).is_some();
        if removed {
            self.record_events(events);
        }
        Ok(removed)
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
        Ok(self.watchlist.read().unwrap().contains_key(&key))
    }
}

#[async_trait]
impl RemoteWatchlistRepository for InMemoryStore {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), DomainError> {
        self.remote_watchlist.write().unwrap().push(entry);
        Ok(())
    }

    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError> {
        self.remote_watchlist
            .write()
            .unwrap()
            .retain(|e| !(e.ap_id == ap_id && e.actor_url == actor_url));
        Ok(())
    }

    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError> {
        Ok(self
            .remote_watchlist
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.actor_url == actor_url)
            .cloned()
            .collect())
    }

    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError> {
        self.remote_watchlist
            .write()
            .unwrap()
            .retain(|e| e.actor_url != actor_url);
        Ok(())
    }

    async fn get_by_derived_uuid(
        &self,
        _uuid: Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError> {
        Ok(self.remote_watchlist.read().unwrap().clone())
    }
}
