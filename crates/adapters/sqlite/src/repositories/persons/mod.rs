use domain::ports::{PersonCommand, PersonQuery};
use sqlx::SqlitePool;
use std::sync::Arc;

mod command;
mod query;
mod rows;

pub struct SqlitePersonAdapter {
    pool: SqlitePool,
}

impl SqlitePersonAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn create_person_adapter(pool: SqlitePool) -> (Arc<dyn PersonCommand>, Arc<dyn PersonQuery>) {
    let adapter = Arc::new(SqlitePersonAdapter::new(pool));
    (
        Arc::clone(&adapter) as Arc<dyn PersonCommand>,
        adapter as Arc<dyn PersonQuery>,
    )
}
