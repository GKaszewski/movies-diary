use domain::{ports::MovieProfileRepository, value_objects::MovieId};
use sqlite::SqliteMovieProfileRepository;
use sqlx::SqlitePool;

async fn pool_with_schema() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE movie_profiles (
            movie_id TEXT PRIMARY KEY, tmdb_id INTEGER, imdb_id TEXT,
            overview TEXT, tagline TEXT, runtime_minutes INTEGER,
            budget_usd INTEGER, revenue_usd INTEGER, vote_average REAL,
            vote_count INTEGER, original_language TEXT, collection_name TEXT,
            enriched_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TABLE movie_genres (movie_id TEXT, tmdb_id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE movie_keywords (movie_id TEXT, tmdb_id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE movie_cast (movie_id TEXT, tmdb_person_id INTEGER,
         name TEXT, character TEXT, billing_order INTEGER, profile_path TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE movie_crew (movie_id TEXT, tmdb_person_id INTEGER,
         name TEXT, job TEXT, department TEXT, profile_path TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn insert_bare_profile(pool: &SqlitePool, movie_id: &str) {
    sqlx::query("INSERT INTO movie_profiles (movie_id, tmdb_id, enriched_at) VALUES (?, 1, ?)")
        .bind(movie_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn null_cast_profile_path_becomes_none_not_empty_string() {
    let pool = pool_with_schema().await;
    let movie_id = MovieId::generate();
    let movie_id_str = movie_id.value().to_string();

    insert_bare_profile(&pool, &movie_id_str).await;

    sqlx::query(
        "INSERT INTO movie_cast (movie_id, tmdb_person_id, name, character, billing_order, profile_path)
         VALUES (?, 1, 'Alice', 'Hero', 0, NULL)",
    )
    .bind(&movie_id_str)
    .execute(&pool)
    .await
    .unwrap();

    let adapter = SqliteMovieProfileRepository::new(pool);
    let profile = adapter.get_by_movie_id(&movie_id).await.unwrap().unwrap();

    assert_eq!(profile.cast.len(), 1);
    assert_eq!(
        profile.cast[0].profile_path, None,
        "NULL profile_path must decode to None, not Some(\"\")"
    );
}

#[tokio::test]
async fn null_budget_usd_becomes_none_not_some_zero() {
    let pool = pool_with_schema().await;
    let movie_id = MovieId::generate();
    let movie_id_str = movie_id.value().to_string();

    insert_bare_profile(&pool, &movie_id_str).await;

    let adapter = SqliteMovieProfileRepository::new(pool);
    let profile = adapter.get_by_movie_id(&movie_id).await.unwrap().unwrap();

    assert_eq!(
        profile.budget_usd, None,
        "NULL budget_usd must decode to None, not Some(0)"
    );
}
