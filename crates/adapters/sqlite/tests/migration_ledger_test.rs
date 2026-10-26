use std::path::Path;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

const LEDGER: &str = include_str!("fixtures/migration_ledger.txt");
const PRE_MERGE_DB: &str = "tests/fixtures/pre_merge.db";

async fn migrate_with_todays_entry_points(pool: &SqlitePool) {
    sqlite::migrate(pool).await.unwrap();
}

async fn ledger(pool: &SqlitePool) -> String {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT version, hex(checksum) FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .unwrap();
    rows.iter().map(|(v, c)| format!("{v} {c}\n")).collect()
}

async fn file_pool(path: &Path, create: bool) -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(create);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap()
}

#[tokio::test]
async fn migrating_a_fresh_database_produces_the_recorded_ledger() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    migrate_with_todays_entry_points(&pool).await;

    assert_eq!(ledger(&pool).await, LEDGER);
}

#[tokio::test]
async fn a_database_migrated_by_the_old_layout_migrates_again_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pre_merge.db");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(PRE_MERGE_DB),
        &path,
    )
    .unwrap();
    let pool = file_pool(&path, false).await;

    sqlite::migrate(&pool).await.unwrap();

    assert_eq!(ledger(&pool).await, LEDGER);
}

async fn wal_pool(path: &Path, create: bool) -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(create)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(opts)
        .await
        .unwrap()
}

async fn race_two_migrators(start: Option<&Path>) {
    for _ in 0..10 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("race.db");
        if let Some(src) = start {
            std::fs::copy(src, &path).unwrap();
        }
        let a = wal_pool(&path, start.is_none()).await;
        let b = wal_pool(&path, false).await;

        let (ra, rb) = tokio::join!(sqlite::migrate(&a), sqlite::migrate(&b));

        ra.unwrap();
        rb.unwrap();
        assert_eq!(ledger(&a).await, LEDGER);
    }
}

#[tokio::test]
async fn two_processes_migrating_the_same_fresh_database_both_succeed() {
    race_two_migrators(None).await;
}

#[tokio::test]
async fn two_processes_migrating_a_database_at_the_old_version_both_succeed() {
    race_two_migrators(Some(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(PRE_MERGE_DB),
    ))
    .await;
}
