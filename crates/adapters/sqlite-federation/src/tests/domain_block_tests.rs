use super::*;
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE blocked_domains (domain TEXT PRIMARY KEY, reason TEXT, blocked_at TEXT NOT NULL)")
        .execute(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn blocked_domain_is_detected() {
    let pool = test_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    assert!(!repo.is_domain_blocked("mastodon.social").await.unwrap());
    repo.add_blocked_domain("mastodon.social", Some("spam")).await.unwrap();
    assert!(repo.is_domain_blocked("mastodon.social").await.unwrap());
}

#[tokio::test]
async fn remove_unblocks_domain() {
    let pool = test_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    repo.add_blocked_domain("spam.xyz", None).await.unwrap();
    repo.remove_blocked_domain("spam.xyz").await.unwrap();
    assert!(!repo.is_domain_blocked("spam.xyz").await.unwrap());
}

#[tokio::test]
async fn get_blocked_domains_returns_all() {
    let pool = test_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    repo.add_blocked_domain("a.com", Some("reason a")).await.unwrap();
    repo.add_blocked_domain("b.com", None).await.unwrap();
    let domains = repo.get_blocked_domains().await.unwrap();
    assert_eq!(domains.len(), 2);
}
