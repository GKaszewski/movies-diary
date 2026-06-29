use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{
        WatchlistEntry, WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    value_objects::{MovieId, UserId},
};

#[async_trait]
pub trait WatchlistRepository: Send + Sync {
    /// Add a new entry. Silently succeeds if the entry already exists.
    async fn add(&self, entry: &WatchlistEntry) -> Result<(), DomainError>;

    /// Remove an entry. Returns NotFound if the entry does not exist.
    async fn remove(&self, user_id: &UserId, movie_id: &MovieId) -> Result<(), DomainError>;

    /// Remove an entry if it exists. Never returns NotFound.
    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
    ) -> Result<bool, DomainError>;

    async fn get_for_user(
        &self,
        user_id: &UserId,
        page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError>;

    async fn contains(&self, user_id: &UserId, movie_id: &MovieId) -> Result<bool, DomainError>;
}
