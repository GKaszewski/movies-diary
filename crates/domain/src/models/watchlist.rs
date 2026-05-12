use chrono::{NaiveDateTime, Utc};

use crate::{
    models::Movie,
    value_objects::{MovieId, UserId, WatchlistEntryId},
};

#[derive(Clone, Debug)]
pub struct WatchlistEntry {
    pub id: WatchlistEntryId,
    pub user_id: UserId,
    pub movie_id: MovieId,
    pub added_at: NaiveDateTime,
}

impl WatchlistEntry {
    pub fn new(user_id: UserId, movie_id: MovieId) -> Self {
        Self {
            id: WatchlistEntryId::generate(),
            user_id,
            movie_id,
            added_at: Utc::now().naive_utc(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WatchlistWithMovie {
    pub entry: WatchlistEntry,
    pub movie: Movie,
}
