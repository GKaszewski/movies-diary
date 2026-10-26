use sqlx::SqlitePool;

mod command;
mod query;
pub(crate) mod rows;

pub struct SqliteMovieRepository {
    pool: SqlitePool,
}

impl SqliteMovieRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
