use super::super::persons::SqlitePersonAdapter;
use domain::{
    models::{ExternalPersonId, Person, PersonId},
    ports::{PersonCommand, PersonQuery},
};
use sqlx::SqlitePool;

async fn pool_with_schema() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE persons (
            id TEXT PRIMARY KEY, external_id TEXT NOT NULL UNIQUE,
            tmdb_person_id INTEGER UNIQUE, name TEXT NOT NULL,
            known_for_department TEXT, profile_path TEXT
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE movies (id TEXT PRIMARY KEY, title TEXT NOT NULL,
         release_year INTEGER, director TEXT, poster_path TEXT,
         external_metadata_id TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE movie_cast (movie_id TEXT, tmdb_person_id INTEGER,
         name TEXT, character TEXT, billing_order INTEGER, profile_path TEXT,
         PRIMARY KEY (movie_id, tmdb_person_id))",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE movie_crew (movie_id TEXT, tmdb_person_id INTEGER,
         name TEXT, job TEXT, department TEXT, profile_path TEXT,
         PRIMARY KEY (movie_id, tmdb_person_id, job))",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

fn make_person(tmdb_id: i64, name: &str, dept: Option<&str>) -> Person {
    let ext = ExternalPersonId::new(format!("tmdb:{tmdb_id}"));
    Person::new(
        PersonId::from_external(&ext),
        ext,
        name.to_string(),
        dept.map(str::to_string),
        None,
    )
}

#[tokio::test]
async fn upsert_batch_inserts_persons() {
    let pool = pool_with_schema().await;
    let adapter = SqlitePersonAdapter::new(pool.clone());

    let persons = vec![
        make_person(1, "Alice", Some("Acting")),
        make_person(2, "Bob", Some("Directing")),
    ];
    adapter.upsert_batch(&persons).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM persons")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn upsert_batch_is_idempotent() {
    let pool = pool_with_schema().await;
    let adapter = SqlitePersonAdapter::new(pool.clone());

    let persons = vec![make_person(1, "Alice", Some("Acting"))];
    adapter.upsert_batch(&persons).await.unwrap();
    adapter.upsert_batch(&persons).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM persons")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn get_by_id_returns_person() {
    let pool = pool_with_schema().await;
    let adapter = SqlitePersonAdapter::new(pool.clone());

    let p = make_person(42, "Charlie", Some("Acting"));
    adapter.upsert_batch(std::slice::from_ref(&p)).await.unwrap();

    let found = adapter.get_by_id(p.id()).await.unwrap().unwrap();
    assert_eq!(found.name(), "Charlie");
    assert_eq!(found.external_id().value(), "tmdb:42");
}

#[tokio::test]
async fn get_by_id_returns_none_for_unknown() {
    let pool = pool_with_schema().await;
    let adapter = SqlitePersonAdapter::new(pool);
    let ext = ExternalPersonId::new("tmdb:999");
    let id = PersonId::from_external(&ext);
    assert!(adapter.get_by_id(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn get_credits_returns_cast_and_crew() {
    let pool = pool_with_schema().await;
    let adapter = SqlitePersonAdapter::new(pool.clone());

    let p = make_person(7, "Diana", Some("Acting"));
    adapter.upsert_batch(std::slice::from_ref(&p)).await.unwrap();

    sqlx::query("INSERT INTO movies VALUES ('m1', 'The Film', 2020, 'Dir', NULL, NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO movie_cast VALUES ('m1', 7, 'Diana', 'Hero', 1, NULL)")
        .execute(&pool)
        .await
        .unwrap();

    let credits = adapter.get_credits(p.id()).await.unwrap();
    assert_eq!(credits.person.name(), "Diana");
    assert_eq!(credits.cast.len(), 1);
    assert_eq!(credits.cast[0].character, "Hero");
    assert!(credits.crew.is_empty());
}
