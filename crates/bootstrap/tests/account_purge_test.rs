mod support;

use std::time::Duration;

use domain::{events::DomainEvent, ports::UserRepository, value_objects::UserId};

async fn wait_for(pool: &sqlx::SqlitePool, sql: &str) -> bool {
    for _ in 0..100 {
        let n: i64 = sqlx::query_scalar(sql).fetch_one(pool).await.unwrap();
        if n > 0 {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn the_worker_purges_a_deleted_account_and_frees_its_email() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let id = UserId::from_uuid(uuid::Uuid::parse_str(&alice.user_id).unwrap());
    let users = sqlite::SqliteUserRepository::new(pool.clone());
    users
        .mark_deleted(
            &id,
            &[DomainEvent::UserDeleted {
                user_id: id.clone(),
            }],
        )
        .await
        .unwrap();

    let worker = bootstrap::build_worker_on(&support::test_config(), pool.clone())
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let running = tokio::spawn(worker.run(stopped));

    let purged = wait_for(
        &pool,
        "SELECT COUNT(*) FROM users WHERE username = 'alice' AND purged_at IS NOT NULL",
    )
    .await;
    stop.send(true).unwrap();
    running.await.unwrap().unwrap();
    assert!(purged, "the worker should purge the tombstoned account");

    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE username = 'alice'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(email, "alice@example.com");
    let again = support::register(&router, "alice2", "alice@example.com").await;
    assert!(!again.user_id.is_empty(), "the email can register again");
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn followers_are_told_before_the_purge_and_the_keypair_survives_it() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let id = UserId::from_uuid(uuid::Uuid::parse_str(&alice.user_id).unwrap());
    for sql in [
        "INSERT INTO ap_remote_actors (url, handle, inbox_url, fetched_at)
         VALUES ('http://127.0.0.1:1/users/ann', 'ann@127.0.0.1', 'http://127.0.0.1:1/inbox', 'now')",
    ] {
        sqlx::query(sql).execute(&pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at)
         VALUES (?, 'http://127.0.0.1:1/users/ann', 'accepted', 'now')",
    )
    .bind(&alice.user_id)
    .execute(&pool)
    .await
    .unwrap();
    let users = sqlite::SqliteUserRepository::new(pool.clone());
    users
        .mark_deleted(
            &id,
            &[DomainEvent::UserDeleted {
                user_id: id.clone(),
            }],
        )
        .await
        .unwrap();

    let worker = bootstrap::build_worker_on(&support::test_config(), pool.clone())
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let running = tokio::spawn(worker.run(stopped));

    let purged = wait_for(
        &pool,
        "SELECT COUNT(*) FROM users WHERE username = 'alice' AND purged_at IS NOT NULL",
    )
    .await;
    let enqueued = wait_for(
        &pool,
        "SELECT COUNT(*) FROM event_queue WHERE event_type = 'FederationDeliveryRequested'",
    )
    .await;
    stop.send(true).unwrap();
    running.await.unwrap().unwrap();
    assert!(purged);
    assert!(enqueued, "the Delete should be enqueued for the follower");

    let deliveries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_queue WHERE event_type = 'FederationDeliveryRequested'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deliveries, 1, "the Delete was enqueued for the follower");
    let keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ap_local_actors WHERE user_id = ?")
        .bind(&alice.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(keys, 1, "the keypair stays until the tombstone cleanup");
}
