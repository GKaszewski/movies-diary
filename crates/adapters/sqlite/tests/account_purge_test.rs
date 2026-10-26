use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::Utc;
use domain::models::{ProfileField, UserProfile, UserSettings, WatchEvent, WatchEventSource};
use domain::models::{WatchlistEntry, WebhookToken};
use domain::ports::{
    AccountPurger, FollowCommand, FollowWrite, UserProfileFieldsRepository, WatchEventCommand,
    WebhookTokenRepository,
};
use domain::testing::{test_goal, test_movie, test_review, test_user, test_watched_at};
use domain::value_objects::{FollowStatus, InstanceIdentity, UserId};
use sqlite::{
    PAYLOAD_KEYED_USER_COLUMNS, PURGED_USER_TABLES, RETAINED_USER_TABLES, SqliteAccountPurger,
    SqliteProfileFieldsRepository, SqliteSocialRepository, SqliteWatchEventRepository,
    SqliteWebhookTokenRepository, URL_KEYED_USER_COLUMNS,
};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const BASE: &str = "https://md.example";
const REMOTE_ACTOR: &str = "https://remote.example/users/ann";

async fn migrated_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlite::migrate(&pool).await.unwrap();
    pool
}

fn instance() -> InstanceIdentity {
    InstanceIdentity::new(BASE)
}

fn purger(pool: &SqlitePool) -> SqliteAccountPurger {
    SqliteAccountPurger::new(pool.clone(), instance())
}

async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn count_for(pool: &SqlitePool, table: &str, column: &str, user: &UserId) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE {column} = ?"))
        .bind(user.value().to_string())
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn exec(pool: &SqlitePool, sql: &str, binds: &[&str]) {
    let mut q = sqlx::query(sql);
    for b in binds {
        q = q.bind(*b);
    }
    q.execute(pool).await.unwrap();
}

struct Seeded {
    user: UserId,
    other: UserId,
    movie_id: String,
}

async fn seed(pool: &SqlitePool, parts: &sqlite::SqliteWireOutput) -> Seeded {
    let alice = test_user("alice");
    let bob = test_user("bob");
    let a = alice.id().clone();
    let b = bob.id().clone();
    let a_str = a.value().to_string();
    let b_str = b.value().to_string();
    parts.user.save(&alice).await.unwrap();
    parts.user.save(&bob).await.unwrap();
    let movie = test_movie("Heat");
    parts.movie_command.upsert_movie(&movie, &[]).await.unwrap();

    for u in [&a, &b] {
        parts
            .watchlist
            .add(&WatchlistEntry::new(u.clone(), movie.id().clone()), &[])
            .await
            .unwrap();
        parts
            .goal_command
            .save(&test_goal(u, 2024, 50), &[])
            .await
            .unwrap();
        parts
            .user_settings
            .save(&UserSettings::new(u.clone()))
            .await
            .unwrap();
        SqliteProfileFieldsRepository::new(pool.clone())
            .set_fields(
                u,
                vec![ProfileField {
                    name: "Site".into(),
                    value: "https://example.com".into(),
                }],
                &[],
            )
            .await
            .unwrap();
        SqliteWebhookTokenRepository::new(pool.clone())
            .save(&WebhookToken::new(
                u.clone(),
                format!("hash-{}", u.value()),
                WatchEventSource::Jellyfin,
                None,
            ))
            .await
            .unwrap();
        SqliteWatchEventRepository::new(pool.clone())
            .save(
                &WatchEvent::new(
                    u.clone(),
                    "Heat".into(),
                    Some(1995),
                    None,
                    WatchEventSource::Jellyfin,
                    test_watched_at(),
                    None,
                ),
                &[],
            )
            .await
            .unwrap();
        parts
            .review
            .save_review(&test_review(movie.id(), u, 4), &[])
            .await
            .unwrap();
        exec(
            pool,
            "INSERT INTO import_sessions (id, user_id, parsed_data, created_at, expires_at) VALUES (?, ?, '{}', 'now', 'later')",
            &[&uuid::Uuid::new_v4().to_string(), &u.value().to_string()],
        )
        .await;
        exec(
            pool,
            "INSERT INTO import_profiles (id, user_id, name, field_mappings, created_at) VALUES (?, ?, 'p', '{}', 'now')",
            &[&uuid::Uuid::new_v4().to_string(), &u.value().to_string()],
        )
        .await;
        exec(
            pool,
            "INSERT INTO refresh_sessions (id, user_id, token, expires_at, created_at) VALUES (?, ?, ?, 'later', 'now')",
            &[
                &uuid::Uuid::new_v4().to_string(),
                &u.value().to_string(),
                &uuid::Uuid::new_v4().to_string(),
            ],
        )
        .await;
        exec(
            pool,
            "INSERT INTO wrap_up_records (id, user_id, start_date, end_date) VALUES (?, ?, 'a', 'b')",
            &[&uuid::Uuid::new_v4().to_string(), &u.value().to_string()],
        )
        .await;
        exec(
            pool,
            "INSERT INTO blocked_actors (local_user_id, remote_actor_url, blocked_at) VALUES (?, ?, 'now')",
            &[&u.value().to_string(), REMOTE_ACTOR],
        )
        .await;
        exec(
            pool,
            "INSERT INTO ap_local_actors (user_id, public_key, private_key, created_at) VALUES (?, 'pub', 'priv', 'now')",
            &[&u.value().to_string()],
        )
        .await;
    }
    let social = SqliteSocialRepository::new(pool.clone(), instance());
    let alice_actor = instance().actor_url_for(&a);
    social
        .apply(
            &[
                FollowWrite::AddFollow {
                    follower_id: a.value(),
                    target_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollower {
                    local_user_id: a.value(),
                    follower_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollow {
                    follower_id: b.value(),
                    target_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollower {
                    local_user_id: b.value(),
                    follower_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollow {
                    follower_id: b.value(),
                    target_actor_url: alice_actor.clone(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollower {
                    local_user_id: b.value(),
                    follower_actor_url: alice_actor,
                    status: FollowStatus::Accepted,
                },
            ],
            &[],
        )
        .await
        .unwrap();
    exec(
        pool,
        "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url)
         SELECT ?, movie_id, ?, rating, comment, watched_at, created_at, ? FROM reviews WHERE user_id = ? LIMIT 1",
        &[
            &uuid::Uuid::new_v4().to_string(),
            &a_str,
            REMOTE_ACTOR,
            &a_str,
        ],
    )
    .await;
    let alice_review: String =
        sqlx::query_scalar("SELECT id FROM reviews WHERE user_id = ? AND remote_actor_url IS NULL")
            .bind(&a_str)
            .fetch_one(pool)
            .await
            .unwrap();
    let bob_review: String = sqlx::query_scalar("SELECT id FROM reviews WHERE user_id = ?")
        .bind(&b_str)
        .fetch_one(pool)
        .await
        .unwrap();
    for (id, object, actor) in [
        (
            "an1",
            format!("{BASE}/reviews/{alice_review}"),
            REMOTE_ACTOR,
        ),
        (
            "an2",
            format!("{BASE}/users/{a_str}/goals/2024"),
            REMOTE_ACTOR,
        ),
        ("an3", format!("{BASE}/reviews/{bob_review}"), REMOTE_ACTOR),
    ] {
        exec(
            pool,
            "INSERT INTO ap_announces (id, object_url, actor_url, announced_at) VALUES (?, ?, ?, 'now')",
            &[id, &object, actor],
        )
        .await;
    }
    for (event_type, status, payload) in [
        (
            "ReviewLogged",
            "done",
            format!(r#"{{"user_id":"{a_str}","comment":"secret"}}"#),
        ),
        (
            "FederationDeliveryRequested",
            "done",
            format!(
                r#"{{"signing_actor_id":"{a_str}","activity_json":{{"type":"Create","content":"secret"}}}}"#
            ),
        ),
        (
            "FederationDeliveryRequested",
            "pending",
            format!(
                r#"{{"signing_actor_id":"{a_str}","activity_json":{{"type":"Delete","id":"x"}}}}"#
            ),
        ),
        (
            "UserDeleted",
            "processing",
            format!(r#"{{"user_id":"{a_str}"}}"#),
        ),
        (
            "ReviewLogged",
            "done",
            format!(r#"{{"user_id":"{b_str}"}}"#),
        ),
        (
            "FollowRequested",
            "pending",
            format!(r#"{{"follower_id":"{b_str}","target":"{a_str}"}}"#),
        ),
        (
            "ActorBlocked",
            "processing",
            format!(r#"{{"user_id":"{b_str}","actor":"{a_str}"}}"#),
        ),
        (
            "Unfollowed",
            "done",
            format!(r#"{{"follower_id":"{b_str}","target":"{a_str}"}}"#),
        ),
    ] {
        sqlx::query("INSERT INTO event_queue (event_type, payload, status) VALUES (?, ?, ?)")
            .bind(event_type)
            .bind(payload)
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
    }
    Seeded {
        user: a,
        other: b,
        movie_id: movie.id().value().to_string(),
    }
}

async fn tombstone(pool: &SqlitePool, user: &UserId) {
    exec(
        pool,
        "UPDATE users SET deleted_at = '2026-01-01T00:00:00Z' WHERE id = ?",
        &[&user.value().to_string()],
    )
    .await;
}

#[tokio::test]
async fn purge_removes_every_user_owned_row() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    for (table, column) in PURGED_USER_TABLES {
        assert!(
            count_for(&pool, table, column, &s.user).await > 0,
            "{table} should be seeded"
        );
    }
    tombstone(&pool, &s.user).await;

    purger(&pool).purge(&s.user).await.unwrap();

    for (table, column) in PURGED_USER_TABLES {
        let left = if *table == "reviews" { 1 } else { 0 };
        assert_eq!(
            count_for(&pool, table, column, &s.user).await,
            left,
            "{table} should be purged"
        );
    }
    let alice_actor = instance().actor_url_for(&s.user);
    for table in ["ap_followers", "ap_following"] {
        let pointing: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {table} WHERE remote_actor_url = ?"
        ))
        .bind(&alice_actor)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pointing, 0, "{table} rows pointing at the user are gone");
    }
    let reviews: i64 = count(
        &pool,
        &format!(
            "SELECT COUNT(*) FROM reviews WHERE user_id = '{}'",
            s.user.value()
        ),
    )
    .await;
    assert_eq!(reviews, 1, "the remote review is not the user's to purge");
    let announces: Vec<String> = sqlx::query_scalar("SELECT id FROM ap_announces ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(announces, vec!["an3"], "only bob's announce survives");

    let queue: Vec<(String, String)> =
        sqlx::query_as("SELECT event_type, status FROM event_queue ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        queue,
        vec![
            (
                "FederationDeliveryRequested".to_string(),
                "pending".to_string()
            ),
            ("UserDeleted".to_string(), "processing".to_string()),
            ("ReviewLogged".to_string(), "done".to_string()),
            ("FollowRequested".to_string(), "pending".to_string()),
            ("ActorBlocked".to_string(), "processing".to_string()),
        ],
        "finished rows mentioning the user are gone; pending/processing rows (incl. other users' events about her) and bob's own rows stay"
    );

    for (table, column) in PURGED_USER_TABLES {
        assert!(
            count_for(&pool, table, column, &s.other).await > 0,
            "{table} of the other user must survive"
        );
    }
    for table in RETAINED_USER_TABLES {
        assert_eq!(count_for(&pool, table, "user_id", &s.user).await, 1);
    }
}

#[tokio::test]
async fn the_purge_table_lists_cover_every_user_referencing_column_in_the_schema() {
    let pool = migrated_pool().await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT m.name, c.name FROM sqlite_master m, pragma_table_info(m.name) c
         WHERE m.type = 'table' AND m.name NOT LIKE 'sqlite_%' AND m.name != 'users'
           AND (c.name LIKE '%user_id%' OR c.name LIKE 'owner%')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let fk_tables: Vec<String> = sqlx::query_scalar(
        "SELECT m.name FROM sqlite_master m, pragma_foreign_key_list(m.name) f
         WHERE m.type = 'table' AND f.\"table\" = 'users'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let found: BTreeSet<(String, String)> = rows.into_iter().collect();
    let mut covered: BTreeSet<(String, String)> = PURGED_USER_TABLES
        .iter()
        .map(|(t, c)| (t.to_string(), c.to_string()))
        .collect();
    covered.extend(
        RETAINED_USER_TABLES
            .iter()
            .map(|t| (t.to_string(), "user_id".to_string())),
    );
    assert_eq!(
        found, covered,
        "a table references a user but the purge neither empties nor retains it"
    );
    for t in fk_tables {
        assert!(
            covered.iter().any(|(table, _)| *table == t),
            "{t} has a foreign key to users but is not in the purge lists"
        );
    }
    for (table, column) in URL_KEYED_USER_COLUMNS
        .iter()
        .chain(PAYLOAD_KEYED_USER_COLUMNS)
    {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_info(?) WHERE name = ?")
            .bind(table)
            .bind(column)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1, "{table}.{column} is matched by the purge but missing");
    }
}

#[tokio::test]
async fn ap_activities_holds_nothing_attributable_to_a_local_user() {
    let pool = migrated_pool().await;
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('ap_activities') ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(columns, vec!["id", "processed_at"]);
}

#[tokio::test]
async fn purge_keeps_movies_and_metadata() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    let movies = count(&pool, "SELECT COUNT(*) FROM movies").await;
    tombstone(&pool, &s.user).await;

    purger(&pool).purge(&s.user).await.unwrap();

    assert_eq!(count(&pool, "SELECT COUNT(*) FROM movies").await, movies);
    assert_eq!(
        count(
            &pool,
            &format!("SELECT COUNT(*) FROM movies WHERE id = '{}'", s.movie_id)
        )
        .await,
        1
    );
}

#[tokio::test]
async fn purge_strips_the_user_row_but_keeps_the_username_and_state() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    parts
        .user
        .update_profile(
            &s.user,
            &UserProfile {
                display_name: Some("Alice".into()),
                bio: Some("bio".into()),
                avatar_path: Some("avatars/a.webp".into()),
                banner_path: Some("banners/a.webp".into()),
                also_known_as: Some("https://x.example/a".into()),
                ..Default::default()
            },
            &[],
        )
        .await
        .unwrap();
    parts.user.mark_deleted(&s.user, &[]).await.unwrap();
    let p = purger(&pool);
    assert!(!p.is_purged(&s.user).await.unwrap());

    p.purge(&s.user).await.unwrap();

    assert!(p.is_purged(&s.user).await.unwrap());
    let tomb = parts
        .user
        .find_tombstoned_by_id(&s.user)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tomb.username().value(), "alice");
    assert_eq!(tomb.display_name(), None);
    assert_eq!(tomb.bio(), None);
    assert_eq!(tomb.also_known_as(), None);
    assert_ne!(tomb.email().value(), "alice@example.com");
    assert!(
        parts
            .user
            .is_username_retired(tomb.username())
            .await
            .unwrap()
    );
    let mut keys = p.owned_image_keys(&s.user).await.unwrap();
    keys.sort();
    assert_eq!(keys, vec!["avatars/a.webp", "banners/a.webp"]);
    p.forget_images(&s.user).await.unwrap();
    assert!(p.owned_image_keys(&s.user).await.unwrap().is_empty());
}

#[tokio::test]
async fn purging_twice_is_a_no_op() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    parts.user.mark_deleted(&s.user, &[]).await.unwrap();
    let p = purger(&pool);
    p.purge(&s.user).await.unwrap();
    let before: i64 = count(&pool, "SELECT COUNT(*) FROM reviews").await;
    let stamp: String = sqlx::query_scalar("SELECT purged_at FROM users WHERE id = ?")
        .bind(s.user.value().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

    p.purge(&s.user).await.unwrap();

    assert_eq!(count(&pool, "SELECT COUNT(*) FROM reviews").await, before);
    let again: String = sqlx::query_scalar("SELECT purged_at FROM users WHERE id = ?")
        .bind(s.user.value().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stamp, again);
    p.purge(&UserId::generate()).await.unwrap();
}

#[tokio::test]
async fn purge_never_touches_a_live_account() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;

    purger(&pool).purge(&s.user).await.unwrap();

    for (table, column) in PURGED_USER_TABLES {
        assert!(
            count_for(&pool, table, column, &s.user).await > 0,
            "{table}"
        );
    }
    assert!(parts.user.find_by_id(&s.user).await.unwrap().is_some());
}

#[tokio::test]
async fn purge_frees_the_email_but_the_username_stays_retired() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    let again = test_user("alice");
    assert!(
        parts.user.save(&again).await.is_err(),
        "email is held while tombstoned"
    );
    parts.user.mark_deleted(&s.user, &[]).await.unwrap();
    assert!(parts.user.save(&again).await.is_err());

    purger(&pool).purge(&s.user).await.unwrap();

    let reuse_email = domain::models::User::new(
        domain::value_objects::Email::new("alice@example.com".into()).unwrap(),
        domain::value_objects::Username::new("alice2".into()).unwrap(),
        domain::value_objects::PasswordHash::new("h".into()).unwrap(),
        domain::models::UserRole::Standard,
    );
    parts.user.save(&reuse_email).await.unwrap();
    assert!(
        parts
            .user
            .is_username_retired(&domain::value_objects::Username::new("alice".into()).unwrap())
            .await
            .unwrap()
    );
    assert!(
        parts.user.save(&again).await.is_err(),
        "the username is never reusable"
    );
}

#[tokio::test]
async fn tombstone_cleanup_removes_rows_older_than_retention_and_keeps_younger_ones() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    parts.user.mark_deleted(&s.user, &[]).await.unwrap();
    let p = purger(&pool);
    p.purge(&s.user).await.unwrap();
    let now = Utc::now().naive_utc();
    let stamp = |days: i64| {
        (now - chrono::Duration::days(days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    };
    exec(
        &pool,
        "UPDATE users SET purged_at = ? WHERE id = ?",
        &[&stamp(8), &s.user.value().to_string()],
    )
    .await;

    let removed = p
        .purge_expired_tombstones(now - chrono::Duration::days(9))
        .await
        .unwrap();
    assert_eq!(removed, 0);
    assert_eq!(count_for(&pool, "users", "id", &s.user).await, 1);

    let alice = s.user.value().to_string();
    let unfinished = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM event_queue WHERE status IN ('pending', 'processing')",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let before = unfinished().await;
    assert!(before >= 3);
    let removed = p
        .purge_expired_tombstones(now - chrono::Duration::days(7))
        .await
        .unwrap();
    assert_eq!(removed, 0, "an unfinished event still names the user");
    assert_eq!(count_for(&pool, "users", "id", &s.user).await, 1);
    assert_eq!(unfinished().await, before);

    exec(
        &pool,
        "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at)
         VALUES (?, 'https://late.example/users/zed', 'pending', '2026-01-01T00:00:00Z')",
        &[&alice],
    )
    .await;
    exec(&pool, "UPDATE event_queue SET status = 'done'", &[]).await;
    let removed = p
        .purge_expired_tombstones(now - chrono::Duration::days(7))
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert_eq!(count_for(&pool, "users", "id", &s.user).await, 0);
    let left: Vec<String> = sqlx::query_scalar("SELECT payload FROM event_queue")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(
        left.iter().all(|p| !p.contains(&alice)),
        "no history of the user remains: {left:?}"
    );
    assert_eq!(
        count_for(&pool, "ap_followers", "local_user_id", &s.user).await,
        0,
        "late shared-inbox follower row"
    );
    assert_eq!(
        count_for(&pool, "ap_local_actors", "user_id", &s.user).await,
        0
    );
    assert_eq!(count_for(&pool, "users", "id", &s.other).await, 1);
    assert_eq!(
        count_for(&pool, "ap_local_actors", "user_id", &s.other).await,
        1
    );
    let retired: i64 = count(
        &pool,
        "SELECT COUNT(*) FROM retired_usernames WHERE username = 'alice'",
    )
    .await;
    assert_eq!(retired, 1, "the username stays retired for good");
}

#[tokio::test]
async fn purge_redacts_the_user_from_global_wrap_up_reports_only() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let s = seed(&pool, &parts).await;
    let (a, b) = (s.user.value().to_string(), s.other.value().to_string());
    let report = |id: &str, name: &str| {
        format!(
            r#"{{"total_watched":3,"most_active_user":{{"user_id":"{id}","display_name":"{name}"}},"total_users_active":2}}"#
        )
    };
    for (rid, id, name) in [("g-alice", &a, "Zelda Quux"), ("g-bob", &b, "Bobby Tables")] {
        exec(
            &pool,
            "INSERT INTO wrap_up_records (id, user_id, start_date, end_date, status, report_json)
             VALUES (?, NULL, '2024-01-01', '2024-12-31', 'completed', ?)",
            &[rid, &report(id, name)],
        )
        .await;
    }
    tombstone(&pool, &s.user).await;

    purger(&pool).purge(&s.user).await.unwrap();

    let all: Vec<(String, String)> =
        sqlx::query_as("SELECT id, report_json FROM wrap_up_records WHERE user_id IS NULL")
            .fetch_all(&pool)
            .await
            .unwrap();
    for (_, json) in &all {
        assert!(!json.contains(&a), "user id left in {json}");
        assert!(!json.contains("Zelda Quux"), "display name left in {json}");
    }
    let alice_row = all.iter().find(|(id, _)| id == "g-alice").unwrap();
    let v: serde_json::Value = serde_json::from_str(&alice_row.1).unwrap();
    assert!(v["most_active_user"].is_null(), "{v}");
    assert_eq!(v["total_watched"], 3, "rest of the report is kept");
    let bob_row = all.iter().find(|(id, _)| id == "g-bob").unwrap();
    assert_eq!(
        bob_row.1,
        report(&b, "Bobby Tables"),
        "other user untouched"
    );
}
