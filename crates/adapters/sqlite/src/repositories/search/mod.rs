use domain::ports::{SearchCommand, SearchPort};
use sqlx::SqlitePool;
use std::sync::Arc;

mod command;
mod query;

pub struct SqliteSearchAdapter {
    pool: SqlitePool,
}

impl SqliteSearchAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn create_search_adapter(pool: SqlitePool) -> (Arc<dyn SearchCommand>, Arc<dyn SearchPort>) {
    let adapter = Arc::new(SqliteSearchAdapter::new(pool));
    (
        Arc::clone(&adapter) as Arc<dyn SearchCommand>,
        adapter as Arc<dyn SearchPort>,
    )
}
