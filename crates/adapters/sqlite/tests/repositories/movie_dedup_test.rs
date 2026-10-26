use std::str::FromStr;

use domain::{
    models::Movie,
    ports::MovieDeduplicator,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterPath, ReleaseYear},
};
use sqlite::SqliteMovieDeduplicator;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

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

async fn insert_movie(pool: &SqlitePool, id: &MovieId, ext: Option<&str>, poster: Option<&str>) {
    sqlx::query(
        "INSERT INTO movies (id, external_metadata_id, title, release_year, poster_path)
         VALUES (?, ?, 'Heat', 1995, ?)",
    )
    .bind(id.value().to_string())
    .bind(ext)
    .bind(poster)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_review(pool: &SqlitePool, movie: &MovieId) {
    sqlx::query(
        "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at)
         VALUES (?, ?, ?, 4, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(movie.value().to_string())
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
}

fn canonical_from(old: &Movie, canonical_id: MovieId) -> Movie {
    Movie::from_persistence(
        canonical_id,
        old.external_metadata_id().cloned(),
        old.title().clone(),
        old.release_year().clone(),
        old.director().map(str::to_string),
        old.poster_path().cloned(),
    )
}

fn old_movie(id: &MovieId, ext: &ExternalMetadataId, poster: Option<&str>) -> Movie {
    Movie::from_persistence(
        id.clone(),
        Some(ext.clone()),
        MovieTitle::new("Heat".into()).unwrap(),
        ReleaseYear::new(1995).unwrap(),
        None,
        poster.map(|p| PosterPath::new(p.into()).unwrap()),
    )
}

async fn scalar(pool: &SqlitePool, sql: &str, bind: &str) -> Option<String> {
    sqlx::query_scalar(sql)
        .bind(bind)
        .fetch_optional(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn merging_into_a_canonical_movie_that_does_not_exist_yet_succeeds() {
    let pool = migrated_pool().await;
    let ext = ExternalMetadataId::new("tmdb:603".into()).unwrap();
    let old_id = MovieId::generate();
    insert_movie(&pool, &old_id, Some("tmdb:603"), None).await;
    insert_review(&pool, &old_id).await;

    let canonical_id = MovieId::from_external(&ext);
    let canonical = canonical_from(&old_movie(&old_id, &ext, None), canonical_id.clone());

    let repointed = SqliteMovieDeduplicator::new(pool.clone())
        .merge_into_canonical(&old_id, &canonical)
        .await
        .unwrap();

    assert_eq!(repointed, 1);
    let ext_of_canonical = scalar(
        &pool,
        "SELECT external_metadata_id FROM movies WHERE id = ?",
        &canonical_id.value().to_string(),
    )
    .await;
    assert_eq!(ext_of_canonical.as_deref(), Some("tmdb:603"));
    assert!(
        scalar(
            &pool,
            "SELECT id FROM movies WHERE id = ?",
            &old_id.value().to_string()
        )
        .await
        .is_none()
    );
    let on_canonical: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE movie_id = ?")
        .bind(canonical_id.value().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(on_canonical, 1);
}

#[tokio::test]
async fn merging_into_an_existing_canonical_movie_moves_the_external_id_and_keeps_its_poster() {
    let pool = migrated_pool().await;
    let ext = ExternalMetadataId::new("tmdb:603".into()).unwrap();
    let old_id = MovieId::generate();
    let canonical_id = MovieId::from_external(&ext);
    insert_movie(&pool, &old_id, Some("tmdb:603"), Some("old.jpg")).await;
    insert_movie(&pool, &canonical_id, None, Some("canonical.jpg")).await;
    insert_review(&pool, &old_id).await;

    let canonical = Movie::from_persistence(
        canonical_id.clone(),
        Some(ext.clone()),
        MovieTitle::new("Heat".into()).unwrap(),
        ReleaseYear::new(1995).unwrap(),
        None,
        Some(PosterPath::new("canonical.jpg".into()).unwrap()),
    );

    SqliteMovieDeduplicator::new(pool.clone())
        .merge_into_canonical(&old_id, &canonical)
        .await
        .unwrap();

    let id = canonical_id.value().to_string();
    assert_eq!(
        scalar(&pool, "SELECT poster_path FROM movies WHERE id = ?", &id)
            .await
            .as_deref(),
        Some("canonical.jpg")
    );
    assert_eq!(
        scalar(
            &pool,
            "SELECT external_metadata_id FROM movies WHERE id = ?",
            &id
        )
        .await
        .as_deref(),
        Some("tmdb:603")
    );
}
