use chrono::Utc;
use k_ap::AnnounceRepository;
use sqlite_federation::SqliteFederationRepository;
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE ap_announces (id TEXT PRIMARY KEY, object_url TEXT NOT NULL, actor_url TEXT NOT NULL, announced_at TEXT NOT NULL)")
        .execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn added_announce_is_counted() {
    let pool = test_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    repo.add_announce(
        "https://remote/ann/1",
        "https://local/r/1",
        "https://remote/u/1",
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(repo.count_announces("https://local/r/1").await.unwrap(), 1);
}

#[tokio::test]
async fn duplicate_announce_is_ignored() {
    let pool = test_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    repo.add_announce(
        "https://remote/ann/1",
        "https://local/r/1",
        "https://remote/u/1",
        Utc::now(),
    )
    .await
    .unwrap();
    repo.add_announce(
        "https://remote/ann/1",
        "https://local/r/1",
        "https://remote/u/1",
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(repo.count_announces("https://local/r/1").await.unwrap(), 1);
}
