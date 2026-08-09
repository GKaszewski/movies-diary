use super::*;
use domain::ports::{FederationAdminQuery, FollowQuery};
use domain::value_objects::{FollowStatus, InstanceIdentity, SocialIdentity};
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    for ddl in [
        "CREATE TABLE ap_following (local_user_id TEXT NOT NULL, remote_actor_url TEXT NOT NULL,
            follow_activity_id TEXT NOT NULL, created_at TEXT, status TEXT NOT NULL,
            PRIMARY KEY (local_user_id, remote_actor_url))",
        "CREATE TABLE ap_followers (local_user_id TEXT NOT NULL, remote_actor_url TEXT NOT NULL,
            follow_activity_id TEXT NOT NULL, created_at TEXT, status TEXT NOT NULL,
            PRIMARY KEY (local_user_id, remote_actor_url))",
        "CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT NOT NULL,
            display_name TEXT, avatar_path TEXT)",
        "CREATE TABLE ap_remote_actors (url TEXT PRIMARY KEY, handle TEXT NOT NULL,
            display_name TEXT, avatar_url TEXT)",
    ] {
        sqlx::query(ddl).execute(&pool).await.unwrap();
    }
    pool
}

fn repo(pool: SqlitePool) -> SqliteSocialRepository {
    SqliteSocialRepository::new(pool, InstanceIdentity::new("https://md.example"))
}

#[tokio::test]
async fn get_relation_returns_no_edges_for_strangers() {
    let r = repo(test_pool().await);
    let rel = r
        .get_relation(uuid::Uuid::new_v4(), "https://other.example/users/bob")
        .await
        .unwrap();
    assert_eq!(rel.following, None);
    assert_eq!(rel.followed_by, None);
}

#[tokio::test]
async fn get_relation_reads_following_direction_only_from_ap_following() {
    let pool = test_pool().await;
    let viewer = uuid::Uuid::new_v4();
    let target = "https://other.example/users/bob";
    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, ?2, '', 'pending')",
    )
    .bind(viewer.to_string())
    .bind(target)
    .execute(&pool)
    .await
    .unwrap();

    let rel = repo(pool).get_relation(viewer, target).await.unwrap();

    assert_eq!(rel.following, Some(FollowStatus::Pending));
    assert_eq!(
        rel.followed_by, None,
        "an ap_following row must not populate followed_by"
    );
}

#[tokio::test]
async fn get_relation_reads_followed_by_from_ap_followers_including_rejected() {
    let pool = test_pool().await;
    let owner = uuid::Uuid::new_v4();
    let requester = "https://other.example/users/carol";
    sqlx::query(
        "INSERT INTO ap_followers (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, ?2, '', 'rejected')",
    )
    .bind(owner.to_string())
    .bind(requester)
    .execute(&pool)
    .await
    .unwrap();

    let rel = repo(pool).get_relation(owner, requester).await.unwrap();

    assert_eq!(rel.followed_by, Some(FollowStatus::Rejected));
    assert_eq!(rel.following, None);
}

#[tokio::test]
async fn get_relation_treats_unknown_status_as_no_edge() {
    let pool = test_pool().await;
    let viewer = uuid::Uuid::new_v4();
    let target = "https://other.example/users/dave";
    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, ?2, '', 'not-a-real-status')",
    )
    .bind(viewer.to_string())
    .bind(target)
    .execute(&pool)
    .await
    .unwrap();

    let rel = repo(pool).get_relation(viewer, target).await.unwrap();

    assert_eq!(rel.following, None);
}

#[tokio::test]
async fn get_pending_following_returns_only_pending_rows() {
    let pool = test_pool().await;
    let viewer = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, 'https://other.example/users/pending', '', 'pending'),
                (?1, 'https://other.example/users/accepted', '', 'accepted')",
    )
    .bind(viewer.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let actors = FollowQuery::get_pending_following(&repo(pool), viewer)
        .await
        .unwrap();

    assert_eq!(
        actors.len(),
        1,
        "accepted rows must not appear in pending_following"
    );
    // The handle-fallback-on-join-miss behavior is covered by
    // `remote_actor_with_no_cached_row_falls_back_to_its_actor_url`; this test
    // only needs to check pending-row filtering, so it asserts on identity.
    assert_eq!(
        actors[0].identity,
        SocialIdentity::Remote {
            actor_url: "https://other.example/users/pending".to_string()
        }
    );
}

#[tokio::test]
async fn remote_actor_with_no_cached_row_falls_back_to_its_actor_url() {
    let pool = test_pool().await;
    let viewer = uuid::Uuid::new_v4();
    let orphan = "https://other.example/users/uncached";
    sqlx::query(
        "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, ?2, '', 'pending')",
    )
    .bind(viewer.to_string())
    .bind(orphan)
    .execute(&pool)
    .await
    .unwrap();
    // deliberately NO ap_remote_actors row for `orphan`

    let actors = FollowQuery::get_pending_following(&repo(pool), viewer)
        .await
        .unwrap();

    assert_eq!(actors.len(), 1);
    assert_eq!(
        actors[0].handle, orphan,
        "with no cached actor, handle must fall back to the actor url, not render empty"
    );
}

#[tokio::test]
async fn count_pending_followers_counts_only_pending() {
    let pool = test_pool().await;
    let owner = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ap_followers (local_user_id, remote_actor_url, follow_activity_id, status)
         VALUES (?1, 'https://other.example/users/a', '', 'pending'),
                (?1, 'https://other.example/users/b', '', 'pending'),
                (?1, 'https://other.example/users/c', '', 'accepted'),
                (?1, 'https://other.example/users/d', '', 'rejected')",
    )
    .bind(owner.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let n = FollowQuery::count_pending_followers(&repo(pool), owner)
        .await
        .unwrap();

    assert_eq!(
        n, 2,
        "only pending rows count; accepted and rejected must not"
    );
}

async fn setup_admin_query_db(pool: &SqlitePool) {
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
async fn test_list_all_followed_remote_actors_deduplicates() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup_admin_query_db(&pool).await;
    let repo =
        SqliteSocialRepository::new(pool.clone(), InstanceIdentity::new("https://localhost"));
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
