use super::*;
use chrono::Utc;
use domain::ports::SocialQueryPort;
use k_ap::ActorRepository;
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE ap_announces (id TEXT PRIMARY KEY, object_url TEXT NOT NULL, actor_url TEXT NOT NULL, announced_at TEXT NOT NULL)")
        .execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn add_announce_stores_and_counts() {
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

async fn setup_db(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ap_remote_actors (
            url TEXT PRIMARY KEY,
            handle TEXT NOT NULL,
            inbox_url TEXT NOT NULL,
            shared_inbox_url TEXT,
            display_name TEXT,
            avatar_url TEXT,
            fetched_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ap_following (
            local_user_id TEXT NOT NULL,
            remote_actor_url TEXT NOT NULL,
            follow_activity_id TEXT NOT NULL,
            status TEXT NOT NULL,
            PRIMARY KEY (local_user_id, remote_actor_url)
        )",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_accepted_following_urls_returns_only_accepted() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup_db(&pool).await;
    let repo = SqliteFederationRepository::new(pool.clone());
    let user_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?, 'https://other.social/users/alice', 'act1', 'accepted'),
                (?, 'https://other.social/users/bob', 'act2', 'pending')",
    )
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let uid = domain::value_objects::UserId::from_uuid(user_id);
    let urls = repo.get_accepted_following_urls(&uid).await.unwrap();
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0], "https://other.social/users/alice");
}

#[tokio::test]
async fn test_list_all_followed_remote_actors_deduplicates() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup_db(&pool).await;
    let repo = SqliteFederationRepository::new(pool.clone());
    let user1 = uuid::Uuid::new_v4();
    let user2 = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO ap_remote_actors (url, handle, inbox_url, fetched_at, display_name)
         VALUES ('https://other.social/users/alice', 'alice@other.social', 'https://other.social/inbox', '2024-01-01', 'Alice')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?, 'https://other.social/users/alice', 'act1', 'accepted'),
                (?, 'https://other.social/users/alice', 'act2', 'accepted')",
    )
    .bind(user1.to_string())
    .bind(user2.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let actors = repo.list_all_followed_remote_actors().await.unwrap();
    assert_eq!(actors.len(), 1);
    assert_eq!(actors[0].handle, "alice@other.social");
}
