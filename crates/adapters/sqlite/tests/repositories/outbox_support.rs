use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

pub async fn migrated_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlite::migrate(&pool).await.unwrap();
    pool
}

pub async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

pub async fn fail_event_inserts(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TRIGGER fail_event_insert BEFORE INSERT ON event_queue \
         BEGIN SELECT RAISE(ABORT, 'enqueue failed'); END",
    )
    .execute(pool)
    .await
    .unwrap();
}
