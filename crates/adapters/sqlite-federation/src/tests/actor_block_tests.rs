use super::*;
use k_ap::BlocklistRepository;
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT, password_hash TEXT, created_at TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TABLE blocked_actors (local_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, remote_actor_url TEXT NOT NULL, blocked_at TEXT NOT NULL, PRIMARY KEY (local_user_id, remote_actor_url))")
        .execute(&pool).await.unwrap();
    let uid = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?)")
        .bind(&uid)
        .bind("a@b.com")
        .bind("hash")
        .bind("2024-01-01")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn block_and_check_actor() {
    let pool = test_pool().await;
    let user_id = uuid::Uuid::parse_str(
        &sqlx::query_scalar::<_, String>("SELECT id FROM users LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap(),
    )
    .unwrap();
    let repo = SqliteFederationRepository::new(pool);
    let actor_url = "https://mastodon.social/users/alice";
    assert!(!repo.is_actor_blocked(user_id, actor_url).await.unwrap());
    repo.add_blocked_actor(user_id, actor_url).await.unwrap();
    assert!(repo.is_actor_blocked(user_id, actor_url).await.unwrap());
    let list = repo.get_blocked_actors(user_id).await.unwrap();
    assert_eq!(list, vec![actor_url.to_string()]);
    repo.remove_blocked_actor(user_id, actor_url).await.unwrap();
    assert!(!repo.is_actor_blocked(user_id, actor_url).await.unwrap());
}
