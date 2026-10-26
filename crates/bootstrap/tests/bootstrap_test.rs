mod support;

#[tokio::test]
async fn server_builds_on_a_fresh_database() {
    let pool = support::memory_pool().await;
    let router = bootstrap::build_server_on(&support::test_config(), pool)
        .await
        .expect("server builds");

    let (status, _, _) = support::send(&router, support::get("/health", None)).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn worker_builds_on_a_fresh_database() {
    let pool = support::memory_pool().await;
    bootstrap::build_worker_on(&support::test_config(), pool)
        .await
        .expect("worker builds");
}

#[cfg(not(feature = "federation"))]
#[tokio::test]
async fn federation_off_build_wires_local_social() {
    use axum::http::Method;

    let (router, _pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let bob = support::register(&router, "bob", "bob@example.com").await;

    let (status, _, body) = support::send(
        &router,
        support::json_request(
            Method::POST,
            "/api/v1/social/follow",
            Some(&alice.token),
            serde_json::json!({ "handle": "bob@localhost:3000" }),
        ),
    )
    .await;
    assert_eq!(status, 200, "follow: {body}");

    let (status, _, body) = support::send(
        &router,
        axum::http::Request::builder()
            .uri("/api/v1/social/followers/pending")
            .header("authorization", format!("Bearer {}", bob.token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, 200, "pending followers: {body}");
    let followers: serde_json::Value = serde_json::from_str(&body).unwrap();
    let actors = followers["actors"].as_array().unwrap();
    assert_eq!(
        actors.len(),
        1,
        "bob should have one pending follower: {body}"
    );
    assert!(
        body.contains(&alice.user_id) || body.contains("alice"),
        "follower should be alice: {body}"
    );
}

#[tokio::test]
async fn server_publishes_to_the_outbox_even_when_bus_is_nats() {
    let pool = support::memory_pool().await;
    let mut config = support::test_config();
    config.events.bus_backend = "nats".into();
    config.nats.url = Some("nats://127.0.0.1:1".into());
    let router = bootstrap::build_server_on(&config, pool.clone())
        .await
        .expect("server builds without a reachable NATS");
    let session = support::register(&router, "alice", "alice@example.com").await;

    let (status, _, body) = support::send(
        &router,
        support::json_request(
            axum::http::Method::POST,
            "/api/v1/reviews",
            Some(&session.token),
            serde_json::json!({
                "manual_title": "Outbox",
                "manual_release_year": 2001,
                "rating": 4,
                "comment": "queued",
                "watched_at": "2024-01-01"
            }),
        ),
    )
    .await;
    assert_eq!(status, 201, "log review: {body}");

    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_queue WHERE event_type = 'ReviewLogged' AND event_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(queued, 1);
}

#[tokio::test]
async fn handler_names_are_unique() {
    let pool = support::memory_pool().await;
    let worker = bootstrap::build_worker_on(&support::test_config(), pool)
        .await
        .expect("worker builds");

    let names = worker.handler_names();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(
        unique.len(),
        names.len(),
        "duplicate handler name in {names:?}"
    );
    let mut sorted = names.clone();
    sorted.sort_unstable();
    let mut expected = vec![
        "account_purge",
        "image_cleanup",
        "movie_discovery_indexer",
        "movie_enrichment",
        "person_enrichment",
        "poster_sync",
        "search_cleanup",
        "search_reindex",
        "wrapup",
    ];
    if cfg!(feature = "federation") {
        expected.extend(["activitypub", "follow_backfill"]);
    }
    expected.sort_unstable();
    assert_eq!(sorted, expected);
}

async fn queued(pool: &sqlx::SqlitePool, event_type: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_queue WHERE event_type = ? AND event_id IS NOT NULL",
    )
    .bind(event_type)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_local_follow_lands_with_its_event_in_the_outbox() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let _bob = support::register(&router, "bob", "bob@example.com").await;

    let (status, _, body) = support::send(
        &router,
        support::json_request(
            axum::http::Method::POST,
            "/api/v1/social/follow",
            Some(&alice.token),
            serde_json::json!({ "handle": "bob@localhost:3000" }),
        ),
    )
    .await;
    assert_eq!(status, 200, "follow: {body}");

    let following: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ap_following")
        .fetch_one(&pool)
        .await
        .unwrap();
    let followers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ap_followers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((following, followers), (1, 1));
    assert_eq!(queued(&pool, "FollowRequested").await, 1);
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn a_remote_block_publishes_its_event_after_the_k_ap_write() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;

    let (status, _, body) = support::send(
        &router,
        support::json_request(
            axum::http::Method::POST,
            "/api/v1/social/block",
            Some(&alice.token),
            serde_json::json!({ "actor_url": "https://remote.example/users/ann" }),
        ),
    )
    .await;
    assert_eq!(status, 204, "block: {body}");

    let blocked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocked_actors")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(blocked, 1);
    assert_eq!(queued(&pool, "ActorBlocked").await, 1);
}

#[tokio::test]
async fn a_deleted_users_token_is_rejected() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let bearer = |token: &str| {
        axum::http::Request::builder()
            .uri("/api/v1/diary/export?format=json")
            .header("authorization", format!("Bearer {token}"))
            .body(axum::body::Body::empty())
            .unwrap()
    };

    let (status, _, body) = support::send(&router, bearer(&alice.token)).await;
    assert_eq!(status, 200, "token works while the account is live: {body}");
    let (status, _, _) =
        support::send(&router, support::get("/diary/export", Some(&alice.cookie))).await;
    assert_eq!(status, 200, "cookie works while the account is live");

    sqlx::query("UPDATE users SET deleted_at = '2026-01-01T00:00:00Z' WHERE id = ?")
        .bind(&alice.user_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, _, body) = support::send(&router, bearer(&alice.token)).await;
    assert_eq!(status, 401, "json route: {body}");
    let (anon_status, anon_headers, _) =
        support::send(&router, support::get("/diary/export", None)).await;
    let (status, headers, _) =
        support::send(&router, support::get("/diary/export", Some(&alice.cookie))).await;
    assert_eq!(status, anon_status, "classic owner page");
    assert_eq!(headers.get("location"), anon_headers.get("location"));
    assert_eq!(headers.get("location").unwrap(), "/login");
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn actor_document_for_a_tombstone_is_gone() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let ap_get = |path: String| {
        axum::http::Request::builder()
            .uri(path)
            .header("accept", "application/activity+json")
            .body(axum::body::Body::empty())
            .unwrap()
    };
    let paths = |id: &str| {
        [
            format!("/users/{id}"),
            format!("/users/{id}/followers"),
            format!("/users/{id}/following"),
            format!("/users/{id}/outbox"),
            format!("/users/{id}/featured"),
        ]
    };
    let (status, _, body) =
        support::send(&router, ap_get(format!("/users/{}", alice.user_id))).await;
    assert_eq!(status, 200, "live actor: {body}");

    sqlx::query("UPDATE users SET deleted_at = '2026-01-01T00:00:00Z' WHERE id = ?")
        .bind(&alice.user_id)
        .execute(&pool)
        .await
        .unwrap();

    for path in paths(&alice.user_id) {
        let (status, _, body) = support::send(&router, ap_get(path.clone())).await;
        assert_eq!(status, 410, "{path}: {body}");
    }
    let (status, _, _) = support::send(
        &router,
        support::get(&format!("/users/{}", alice.user_id), None),
    )
    .await;
    assert_eq!(status, 404, "the classic profile page stays 404");
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn posting_to_a_tombstones_inbox_is_gone() {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let post = |path: String| {
        axum::http::Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/activity+json")
            .body(axum::body::Body::from("{}"))
            .unwrap()
    };
    let path = format!("/users/{}/inbox", alice.user_id);
    let (live, _, _) = support::send(&router, post(path.clone())).await;
    assert_ne!(live, 410, "a live user's inbox is not gone");

    sqlx::query("UPDATE users SET deleted_at = '2026-01-01T00:00:00Z' WHERE id = ?")
        .bind(&alice.user_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, _, body) = support::send(&router, post(path)).await;
    assert_eq!(status, 410, "{body}");
}
