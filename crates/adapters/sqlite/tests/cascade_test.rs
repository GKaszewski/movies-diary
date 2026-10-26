use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::Utc;
use domain::models::{
    CastMember, CrewMember, Genre, Keyword, Movie, MovieProfile, ProfileField, UserSettings,
    WatchEvent, WatchEventSource, WatchlistEntry, WebhookToken,
};
use domain::ports::{
    FollowCommand, FollowWrite, UserProfileFieldsRepository, WatchEventCommand,
    WebhookTokenRepository,
};
use domain::testing::{
    MOVIE_CASCADE_TABLES, USER_CASCADE_TABLES, USER_CASCADE_TABLES_OUTSIDE_STORE, test_goal,
    test_movie, test_review, test_user, test_watched_at,
};
use domain::value_objects::{FollowStatus, MovieId, UserId};
use sqlite::{
    SqliteProfileFieldsRepository, SqliteSocialRepository, SqliteWatchEventRepository,
    SqliteWebhookTokenRepository,
};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const REMOTE_ACTOR: &str = "https://remote.example/users/ann";

async fn migrated_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let (foreign_keys,): (i64,) = sqlx::query_as("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        foreign_keys, 1,
        "tests must run with FKs enforced like production"
    );
    sqlite::migrate(&pool).await.unwrap();
    pool
}

async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn tables_cascading_from(pool: &SqlitePool, parent: &str) -> BTreeSet<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT m.name FROM sqlite_master m, pragma_foreign_key_list(m.name) f
         WHERE m.type = 'table' AND f.\"table\" = ? AND f.on_delete = 'CASCADE'",
    )
    .bind(parent)
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .collect()
}

fn names(lists: &[&[&str]]) -> BTreeSet<String> {
    lists
        .iter()
        .flat_map(|l| l.iter())
        .map(|t| t.to_string())
        .collect()
}

fn movie_profile(movie_id: &MovieId) -> MovieProfile {
    MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 603,
        imdb_id: None,
        overview: None,
        tagline: None,
        runtime_minutes: None,
        budget_usd: None,
        revenue_usd: None,
        vote_average: None,
        vote_count: None,
        original_language: None,
        collection_name: None,
        genres: vec![Genre {
            tmdb_id: 1,
            name: "Sci-Fi".into(),
        }],
        keywords: vec![Keyword {
            tmdb_id: 2,
            name: "simulation".into(),
        }],
        cast: vec![CastMember {
            tmdb_person_id: 3,
            name: "Actor".into(),
            character: "Role".into(),
            billing_order: 0,
            profile_path: None,
        }],
        crew: vec![CrewMember {
            tmdb_person_id: 4,
            name: "Director".into(),
            job: "Director".into(),
            department: "Directing".into(),
            profile_path: None,
        }],
        enriched_at: Utc::now(),
    }
}

async fn seed_user_with_dependents(
    pool: &SqlitePool,
    parts: &sqlite::SqliteWireOutput,
) -> (UserId, Movie) {
    let user = test_user("alice");
    let user_id = user.id().clone();
    let movie = test_movie("Heat");
    parts.user.save(&user).await.unwrap();
    parts.movie_command.upsert_movie(&movie, &[]).await.unwrap();

    parts
        .watchlist
        .add(
            &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
            &[],
        )
        .await
        .unwrap();
    parts
        .goal_command
        .save(&test_goal(&user_id, 2024, 50), &[])
        .await
        .unwrap();
    parts
        .user_settings
        .save(&UserSettings::new(user_id.clone()))
        .await
        .unwrap();
    SqliteProfileFieldsRepository::new(pool.clone())
        .set_fields(
            &user_id,
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
            user_id.clone(),
            "hash".into(),
            WatchEventSource::Jellyfin,
            None,
        ))
        .await
        .unwrap();
    SqliteWatchEventRepository::new(pool.clone())
        .save(
            &WatchEvent::new(
                user_id.clone(),
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
    let social = SqliteSocialRepository::new(
        pool.clone(),
        domain::value_objects::InstanceIdentity::new("https://md.example"),
    );
    social
        .apply(
            &[
                FollowWrite::AddFollow {
                    follower_id: user_id.value(),
                    target_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
                FollowWrite::AddFollower {
                    local_user_id: user_id.value(),
                    follower_actor_url: REMOTE_ACTOR.into(),
                    status: FollowStatus::Accepted,
                },
            ],
            &[],
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO blocked_actors (local_user_id, remote_actor_url, blocked_at) VALUES (?, ?, 'now')")
        .bind(user_id.value().to_string())
        .bind(REMOTE_ACTOR)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ap_local_actors (user_id, public_key, private_key, created_at) VALUES (?, 'pub', 'priv', 'now')")
        .bind(user_id.value().to_string())
        .execute(pool)
        .await
        .unwrap();

    parts
        .review
        .save_review(&test_review(movie.id(), &user_id, 4), &[])
        .await
        .unwrap();
    (user_id, movie)
}

#[tokio::test]
async fn the_cascade_lists_match_the_foreign_keys_declared_by_the_migrations() {
    let pool = migrated_pool().await;

    assert_eq!(
        tables_cascading_from(&pool, "users").await,
        names(&[USER_CASCADE_TABLES, USER_CASCADE_TABLES_OUTSIDE_STORE]),
    );
    assert_eq!(
        tables_cascading_from(&pool, "movies").await,
        names(&[MOVIE_CASCADE_TABLES]),
    );
}

#[tokio::test]
async fn deleting_a_user_cascades_to_every_table_that_hangs_off_a_user() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let (user_id, _) = seed_user_with_dependents(&pool, &parts).await;
    for table in USER_CASCADE_TABLES
        .iter()
        .chain(USER_CASCADE_TABLES_OUTSIDE_STORE)
    {
        assert!(count(&pool, table).await > 0, "{table} should be seeded");
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id.value().to_string())
        .execute(&pool)
        .await
        .unwrap();

    for table in USER_CASCADE_TABLES
        .iter()
        .chain(USER_CASCADE_TABLES_OUTSIDE_STORE)
    {
        assert_eq!(count(&pool, table).await, 0, "{table} should cascade");
    }
    assert_eq!(count(&pool, "reviews").await, 1, "reviews have no user FK");
}

#[tokio::test]
async fn deleting_a_movie_cascades_to_watchlist_entries_and_every_enrichment_table() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let user = test_user("alice");
    let movie = test_movie("Heat");
    parts.user.save(&user).await.unwrap();
    parts.movie_command.upsert_movie(&movie, &[]).await.unwrap();
    parts
        .watchlist
        .add(
            &WatchlistEntry::new(user.id().clone(), movie.id().clone()),
            &[],
        )
        .await
        .unwrap();
    parts
        .movie_profile
        .upsert(&movie_profile(movie.id()))
        .await
        .unwrap();
    SqliteWatchEventRepository::new(pool.clone())
        .save(
            &WatchEvent::new(
                user.id().clone(),
                "Heat".into(),
                Some(1995),
                None,
                WatchEventSource::Plex,
                test_watched_at(),
                Some(movie.id().clone()),
            ),
            &[],
        )
        .await
        .unwrap();
    for table in MOVIE_CASCADE_TABLES {
        assert!(count(&pool, table).await > 0, "{table} should be seeded");
    }

    parts
        .movie_command
        .delete_movie(movie.id(), &[])
        .await
        .unwrap();

    for table in MOVIE_CASCADE_TABLES {
        assert_eq!(count(&pool, table).await, 0, "{table} should cascade");
    }
    let (movie_id,): (Option<String>,) = sqlx::query_as("SELECT movie_id FROM watch_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(movie_id, None, "watch events are detached, not deleted");
}

#[tokio::test]
async fn deleting_a_movie_that_has_reviews_is_refused_because_reviews_do_not_cascade() {
    let pool = migrated_pool().await;
    let parts = sqlite::wire_pool(pool.clone()).await.unwrap();
    let user = test_user("alice");
    let movie = test_movie("Heat");
    parts.user.save(&user).await.unwrap();
    parts.movie_command.upsert_movie(&movie, &[]).await.unwrap();
    parts
        .review
        .save_review(&test_review(movie.id(), user.id(), 5), &[])
        .await
        .unwrap();

    let result = parts.movie_command.delete_movie(movie.id(), &[]).await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(_))
    ));
    assert_eq!(count(&pool, "movies").await, 1);
    assert_eq!(count(&pool, "reviews").await, 1);
}
