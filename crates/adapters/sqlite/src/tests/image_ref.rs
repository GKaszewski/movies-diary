use super::*;

async fn setup(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            username TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'standard',
            bio TEXT,
            avatar_path TEXT
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS movies (
            id TEXT PRIMARY KEY,
            external_metadata_id TEXT,
            title TEXT NOT NULL,
            release_year INTEGER,
            director TEXT,
            poster_path TEXT
        )",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn list_keys_returns_both_avatar_and_poster_paths() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    sqlx::query("INSERT INTO users VALUES ('u1','e@e.com','u','h','2024-01-01','standard',NULL,'avatars/u1')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO movies VALUES ('m1','tt1','Title',2020,'Dir','posters/m1')")
        .execute(&pool)
        .await
        .unwrap();

    let adapter = SqliteImageRefAdapter::new(pool);
    let mut keys = adapter.list_keys().await.unwrap();
    keys.sort();

    assert_eq!(keys, vec!["avatars/u1", "posters/m1"]);
}

#[tokio::test]
async fn list_keys_excludes_nulls() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    sqlx::query(
        "INSERT INTO users VALUES ('u1','e@e.com','u','h','2024-01-01','standard',NULL,NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let adapter = SqliteImageRefAdapter::new(pool);
    assert_eq!(adapter.list_keys().await.unwrap(), Vec::<String>::new());
}

#[tokio::test]
async fn swap_updates_avatar_path() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    sqlx::query("INSERT INTO users VALUES ('u1','e@e.com','u','h','2024-01-01','standard',NULL,'avatars/u1')")
        .execute(&pool).await.unwrap();

    let adapter = SqliteImageRefAdapter::new(pool.clone());
    adapter.swap("avatars/u1", "avatars/u1.avif").await.unwrap();

    let row: (Option<String>,) = sqlx::query_as("SELECT avatar_path FROM users WHERE id='u1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0.as_deref(), Some("avatars/u1.avif"));
}

#[tokio::test]
async fn swap_updates_poster_path() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    sqlx::query("INSERT INTO movies VALUES ('m1','tt1','Title',2020,'Dir','posters/m1')")
        .execute(&pool)
        .await
        .unwrap();

    let adapter = SqliteImageRefAdapter::new(pool.clone());
    adapter.swap("posters/m1", "posters/m1.avif").await.unwrap();

    let row: (Option<String>,) = sqlx::query_as("SELECT poster_path FROM movies WHERE id='m1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0.as_deref(), Some("posters/m1.avif"));
}

#[tokio::test]
async fn swap_noop_when_key_not_found() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    setup(&pool).await;

    let adapter = SqliteImageRefAdapter::new(pool);
    adapter
        .swap("missing/key", "missing/key.avif")
        .await
        .unwrap();
}
