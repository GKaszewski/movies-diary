use super::create_search_adapter;
use domain::{
    models::{
        collections::PageParams, EntityType, IndexableDocument, Movie, SearchFilters, SearchQuery,
    },
    value_objects::{MovieId, MovieTitle, ReleaseYear},
};
use sqlx::SqlitePool;

async fn pool_with_schema() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE movies (id TEXT PRIMARY KEY, title TEXT NOT NULL,
         release_year INTEGER, director TEXT, poster_path TEXT, external_metadata_id TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE persons (id TEXT PRIMARY KEY, external_id TEXT UNIQUE,
         tmdb_person_id INTEGER UNIQUE, name TEXT NOT NULL,
         known_for_department TEXT, profile_path TEXT)",
    )
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
    sqlx::query("CREATE TABLE movie_genres (movie_id TEXT, tmdb_id INTEGER, name TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE VIRTUAL TABLE movies_fts USING fts5(
            movie_id UNINDEXED, title, director, overview, genres, keywords,
            cast_names, crew_names, release_year UNINDEXED, language UNINDEXED)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE VIRTUAL TABLE people_fts USING fts5(
            person_id UNINDEXED, name, known_for_department UNINDEXED)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

fn test_movie(id: &str, title: &str, year: u16) -> Movie {
    Movie::from_persistence(
        MovieId::from_uuid(uuid::Uuid::parse_str(id).unwrap()),
        None,
        MovieTitle::new(title.into()).unwrap(),
        ReleaseYear::new(year).unwrap(),
        Some("Test Director".to_string()),
        None,
    )
}

fn default_page() -> PageParams {
    PageParams::new(Some(10), Some(0)).unwrap()
}

#[tokio::test]
async fn index_and_search_movie_by_title() {
    let pool = pool_with_schema().await;
    let (cmd, query) = create_search_adapter(pool.clone());

    let id_str = "00000000-0000-0000-0000-000000000001";
    let movie = test_movie(id_str, "Interstellar", 2014);
    let movie_id = movie.id().clone();

    sqlx::query("INSERT INTO movies VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id_str)
        .bind("Interstellar")
        .bind(2014i32)
        .bind("Christopher Nolan")
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await
        .unwrap();

    cmd.index(IndexableDocument::Movie {
        id: movie_id.clone(),
        movie: Box::new(movie),
        profile: None,
    })
    .await
    .unwrap();

    let results = query
        .search(&SearchQuery {
            text: Some("Interstellar".to_string()),
            filters: SearchFilters::default(),
            page: default_page(),
        })
        .await
        .unwrap();

    assert_eq!(results.movies.items.len(), 1);
    assert_eq!(results.movies.items[0].title, "Interstellar");
}

#[tokio::test]
async fn remove_movie_clears_from_index() {
    let pool = pool_with_schema().await;
    let (cmd, query) = create_search_adapter(pool.clone());

    let id_str = "00000000-0000-0000-0000-000000000002";
    let movie = test_movie(id_str, "Inception", 2010);
    let movie_id = movie.id().clone();

    sqlx::query("INSERT INTO movies VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id_str)
        .bind("Inception")
        .bind(2010i32)
        .bind("Christopher Nolan")
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await
        .unwrap();

    cmd.index(IndexableDocument::Movie {
        id: movie_id.clone(),
        movie: Box::new(movie),
        profile: None,
    })
    .await
    .unwrap();
    cmd.remove(EntityType::Movie, id_str).await.unwrap();

    let results = query
        .search(&SearchQuery {
            text: Some("Inception".to_string()),
            filters: SearchFilters::default(),
            page: default_page(),
        })
        .await
        .unwrap();

    assert!(results.movies.items.is_empty());
}

#[tokio::test]
async fn search_with_genre_filter() {
    let pool = pool_with_schema().await;
    let (cmd, query) = create_search_adapter(pool.clone());

    let id_str = "00000000-0000-0000-0000-000000000003";
    let movie = test_movie(id_str, "The Dark Knight", 2008);
    let movie_id = movie.id().clone();

    sqlx::query("INSERT INTO movies VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id_str)
        .bind("The Dark Knight")
        .bind(2008i32)
        .bind("Christopher Nolan")
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO movie_genres VALUES (?, 1, 'Action')")
        .bind(id_str)
        .execute(&pool)
        .await
        .unwrap();

    cmd.index(IndexableDocument::Movie {
        id: movie_id.clone(),
        movie: Box::new(movie),
        profile: None,
    })
    .await
    .unwrap();

    // Matching genre — no text filter
    let results = query
        .search(&SearchQuery {
            text: None,
            filters: SearchFilters {
                genre: Some("Action".to_string()),
                ..Default::default()
            },
            page: default_page(),
        })
        .await
        .unwrap();
    assert_eq!(results.movies.items.len(), 1);

    // Non-matching genre
    let results = query
        .search(&SearchQuery {
            text: None,
            filters: SearchFilters {
                genre: Some("Comedy".to_string()),
                ..Default::default()
            },
            page: default_page(),
        })
        .await
        .unwrap();
    assert!(results.movies.items.is_empty());
}
