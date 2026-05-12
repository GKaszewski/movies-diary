use super::*;
use domain::models::UserRole;
use domain::value_objects::{Email, PasswordHash, Username};
use sqlx::SqlitePool;

async fn setup() -> (SqlitePool, SqliteUserRepository) {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, username TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'standard', bio TEXT, avatar_path TEXT)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let repo = SqliteUserRepository::new(pool.clone());
    (pool, repo)
}

#[tokio::test]
async fn find_by_id_returns_none_when_not_found() {
    let (_, repo) = setup().await;
    let result = repo
        .find_by_id(&UserId::from_uuid(uuid::Uuid::new_v4()))
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn find_by_id_returns_user_when_found() {
    let (pool, repo) = setup().await;
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, username, password_hash, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(id.to_string())
    .bind("test@example.com")
    .bind("test")
    .bind("$argon2id$v=19$m=65536,t=2,p=1$fakesalt$fakehash")
    .bind("2026-01-01T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();

    let result = repo.find_by_id(&UserId::from_uuid(id)).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().email().value(), "test@example.com");
}

#[tokio::test]
async fn update_profile_persists_bio_and_avatar() {
    let (_, repo) = setup().await;
    let user = domain::models::User::new(
        Email::new("test@example.com".to_string()).unwrap(),
        Username::new("testuser".to_string()).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
        UserRole::Standard,
    );
    repo.save(&user).await.unwrap();

    repo.update_profile(
        user.id(),
        Some("My biography".to_string()),
        Some("avatars/user1".to_string()),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.bio(), Some("My biography"));
    assert_eq!(found.avatar_path(), Some("avatars/user1"));
}

#[tokio::test]
async fn update_profile_clears_fields_with_none() {
    let (_, repo) = setup().await;
    let user = domain::models::User::new(
        Email::new("test2@example.com".to_string()).unwrap(),
        Username::new("testuser2".to_string()).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
        UserRole::Standard,
    );
    repo.save(&user).await.unwrap();
    repo.update_profile(user.id(), Some("bio".to_string()), Some("path".to_string()))
        .await
        .unwrap();
    repo.update_profile(user.id(), None, None).await.unwrap();

    let found = repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.bio(), None);
    assert_eq!(found.avatar_path(), None);
}
