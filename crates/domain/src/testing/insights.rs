use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    errors::DomainError, models::insights::WatchRow, ports::InsightsQuery, value_objects::UserId,
};

// ── PanicInsightsQuery ──────────────────────────────────────────────────────

pub struct PanicInsightsQuery;

#[async_trait]
impl InsightsQuery for PanicInsightsQuery {
    async fn get_watch_rows(&self, _: &UserId) -> Result<Vec<WatchRow>, DomainError> {
        panic!("PanicInsightsQuery called")
    }
}

// ── InMemoryInsightsQuery ───────────────────────────────────────────────────

pub struct InMemoryInsightsQuery {
    pub rows_by_user: Mutex<HashMap<uuid::Uuid, Vec<WatchRow>>>,
}

impl InMemoryInsightsQuery {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rows_by_user: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_rows(user_id: uuid::Uuid, rows: Vec<WatchRow>) -> Arc<Self> {
        let query = Self::new();
        query.rows_by_user.lock().unwrap().insert(user_id, rows);
        query
    }
}

#[async_trait]
impl InsightsQuery for InMemoryInsightsQuery {
    async fn get_watch_rows(&self, user_id: &UserId) -> Result<Vec<WatchRow>, DomainError> {
        Ok(self
            .rows_by_user
            .lock()
            .unwrap()
            .get(&user_id.value())
            .cloned()
            .unwrap_or_default())
    }
}
