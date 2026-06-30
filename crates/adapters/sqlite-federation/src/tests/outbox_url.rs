use super::*;
use k_ap::{FollowRepository, FollowingStatus, RemoteActor};

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE ap_remote_actors (
            url TEXT PRIMARY KEY, handle TEXT NOT NULL, inbox_url TEXT NOT NULL,
            shared_inbox_url TEXT, display_name TEXT, avatar_url TEXT,
            outbox_url TEXT, bio TEXT, banner_url TEXT, followers_url TEXT,
            following_url TEXT, also_known_as TEXT, fetched_at TEXT NOT NULL
         );
         CREATE TABLE ap_following (
            local_user_id TEXT NOT NULL, remote_actor_url TEXT NOT NULL,
            follow_activity_id TEXT, created_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            PRIMARY KEY (local_user_id, remote_actor_url)
         );",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

#[tokio::test]
async fn get_following_outbox_url_returns_stored_url() {
    let pool = setup_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    let local_user = uuid::Uuid::new_v4();
    let actor = RemoteActor {
        url: "https://remote.example/users/alice".to_string(),
        handle: "alice@remote.example".to_string(),
        inbox_url: "https://remote.example/users/alice/inbox".to_string(),
        shared_inbox_url: None,
        display_name: None,
        avatar_url: None,
        outbox_url: Some("https://remote.example/users/alice/outbox".to_string()),
        bio: None,
        banner_url: None,
        followers_url: None,
        following_url: None,
        also_known_as: vec![],
        fetched_at: None,
    };
    repo.add_following(local_user, actor, "https://local/activities/1")
        .await
        .unwrap();
    repo.update_following_status(
        local_user,
        "https://remote.example/users/alice",
        FollowingStatus::Accepted,
    )
    .await
    .unwrap();

    let result = repo
        .get_following_outbox_url(local_user, "https://remote.example/users/alice")
        .await
        .unwrap();
    assert_eq!(
        result,
        Some("https://remote.example/users/alice/outbox".to_string())
    );
}

#[tokio::test]
async fn get_following_outbox_url_returns_none_when_not_following() {
    let pool = setup_pool().await;
    let repo = SqliteFederationRepository::new(pool);
    let result = repo
        .get_following_outbox_url(uuid::Uuid::new_v4(), "https://remote.example/users/alice")
        .await
        .unwrap();
    assert_eq!(result, None);
}
