use sqlx::SqlitePool;

mod command;
mod query;
mod rows;
mod webhook_token;

pub use webhook_token::SqliteWebhookTokenRepository;

pub struct SqliteWatchEventRepository {
    pool: SqlitePool,
}

impl SqliteWatchEventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
