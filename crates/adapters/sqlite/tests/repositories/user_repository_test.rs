use domain::models::UserProfile;
use domain::ports::UserRepository;
use domain::testing::test_user;
use domain::value_objects::UserId;
use sqlite::SqliteUserRepository;
use sqlx::SqlitePool;

use super::outbox_support::{count, fail_event_inserts, migrated_pool};

async fn setup() -> (SqlitePool, SqliteUserRepository) {
    let pool = migrated_pool().await;
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
        "INSERT INTO users (id, email, username, password_hash, created_at) VALUES (?, ?, ?, ?, ?)",
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
    let user = test_user("testuser");
    repo.save(&user).await.unwrap();

    repo.update_profile(
        user.id(),
        &UserProfile {
            bio: Some("My biography".to_string()),
            avatar_path: Some("avatars/user1".to_string()),
            ..Default::default()
        },
        &[],
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
    let user = test_user("testuser2");
    repo.save(&user).await.unwrap();
    repo.update_profile(
        user.id(),
        &UserProfile {
            bio: Some("bio".to_string()),
            avatar_path: Some("path".to_string()),
            ..Default::default()
        },
        &[],
    )
    .await
    .unwrap();
    repo.update_profile(user.id(), &UserProfile::default(), &[])
        .await
        .unwrap();

    let found = repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.bio(), None);
    assert_eq!(found.avatar_path(), None);
}

fn deleted_event(user: &domain::models::User) -> domain::events::DomainEvent {
    domain::events::DomainEvent::UserDeleted {
        user_id: user.id().clone(),
    }
}

async fn add_session(pool: &SqlitePool, user_id: &UserId, token: &str) {
    sqlx::query(
        "INSERT INTO refresh_sessions (id, user_id, token, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id.value().to_string())
    .bind(token)
    .bind("2099-01-01T00:00:00+00:00")
    .bind("2026-01-01T00:00:00+00:00")
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn marking_a_user_deleted_hides_them_from_every_lookup() {
    let (pool, repo) = setup().await;
    let admin =
        domain::testing::test_user_with_role(UserId::generate(), domain::models::UserRole::Admin);
    repo.save(&admin).await.unwrap();
    let other = test_user("other");
    repo.save(&other).await.unwrap();
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
        .bind(other.id().value().to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(repo.count_active_admins().await.unwrap(), 2);

    repo.mark_deleted(admin.id(), &[deleted_event(&admin)])
        .await
        .unwrap();

    assert!(repo.find_by_id(admin.id()).await.unwrap().is_none());
    assert!(repo.find_by_email(admin.email()).await.unwrap().is_none());
    assert!(
        repo.find_by_username(admin.username())
            .await
            .unwrap()
            .is_none()
    );
    let listed = repo.list_with_stats().await.unwrap();
    let emails: Vec<_> = listed.iter().map(|u| u.email()).collect();
    assert!(emails.contains(&other.email().value()));
    assert!(!emails.contains(&admin.email().value()));
    assert_eq!(repo.count_active_admins().await.unwrap(), 1);
    assert!(
        repo.find_tombstoned_by_id(admin.id())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.find_tombstoned_by_id(other.id())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn marking_deleted_retires_the_username_and_drops_sessions_atomically() {
    let (pool, repo) = setup().await;
    let user = test_user("leaver");
    let keeper = test_user("keeper");
    repo.save(&user).await.unwrap();
    repo.save(&keeper).await.unwrap();
    add_session(&pool, user.id(), "t1").await;
    add_session(&pool, user.id(), "t2").await;
    add_session(&pool, keeper.id(), "t3").await;

    repo.mark_deleted(user.id(), &[deleted_event(&user)])
        .await
        .unwrap();

    assert!(repo.is_username_retired(user.username()).await.unwrap());
    assert!(!repo.is_username_retired(keeper.username()).await.unwrap());
    assert_eq!(count(&pool, "refresh_sessions").await, 1, "only keeper's");
    assert_eq!(count(&pool, "event_queue").await, 1);
    let deleted_at: Option<String> =
        sqlx::query_scalar("SELECT deleted_at FROM users WHERE id = ?")
            .bind(user.id().value().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(deleted_at.is_some());
}

#[tokio::test]
async fn a_failed_enqueue_rolls_back_tombstone_retirement_and_session_drop() {
    let (pool, repo) = setup().await;
    let user = test_user("stayer");
    repo.save(&user).await.unwrap();
    add_session(&pool, user.id(), "t1").await;
    fail_event_inserts(&pool).await;

    let result = repo.mark_deleted(user.id(), &[deleted_event(&user)]).await;

    assert!(result.is_err());
    assert!(repo.find_by_id(user.id()).await.unwrap().is_some());
    assert!(!repo.is_username_retired(user.username()).await.unwrap());
    assert_eq!(count(&pool, "retired_usernames").await, 0);
    assert_eq!(count(&pool, "refresh_sessions").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 0);
}

#[tokio::test]
async fn retired_names_match_case_insensitively() {
    let (_, repo) = setup().await;
    let user = test_user("MixedCase");
    repo.save(&user).await.unwrap();
    repo.mark_deleted(user.id(), &[deleted_event(&user)])
        .await
        .unwrap();

    let lower = domain::value_objects::Username::new("mixedcase".to_string()).unwrap();
    let upper = domain::value_objects::Username::new("MIXEDCASE".to_string()).unwrap();
    assert!(repo.is_username_retired(&lower).await.unwrap());
    assert!(repo.is_username_retired(&upper).await.unwrap());
}

#[tokio::test]
async fn marking_the_last_admin_deleted_is_refused_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(dir.path().join("u.db"))
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .unwrap();
    sqlite::migrate(&pool).await.unwrap();
    let repo = std::sync::Arc::new(SqliteUserRepository::new(pool.clone()));
    let a = test_user("admin_a");
    let b = test_user("admin_b");
    repo.save(&a).await.unwrap();
    repo.save(&b).await.unwrap();
    sqlx::query("UPDATE users SET role = 'admin' WHERE id IN (?, ?)")
        .bind(a.id().value().to_string())
        .bind(b.id().value().to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(repo.count_active_admins().await.unwrap(), 2);

    let (ra, rb) = (repo.clone(), repo.clone());
    let (ida, idb) = (a.id().clone(), b.id().clone());
    let (ea, eb) = (deleted_event(&a), deleted_event(&b));
    let (x, y) = tokio::join!(
        tokio::spawn(async move { ra.mark_deleted(&ida, &[ea]).await }),
        tokio::spawn(async move { rb.mark_deleted(&idb, &[eb]).await }),
    );
    let results = [x.unwrap(), y.unwrap()];

    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(
        results
            .iter()
            .any(|r| matches!(r, Err(domain::errors::DomainError::Conflict(_)))),
        "{results:?}"
    );
    assert_eq!(repo.count_active_admins().await.unwrap(), 1);
    assert_eq!(
        count(&pool, "event_queue").await,
        1,
        "loser enqueued nothing"
    );
}
