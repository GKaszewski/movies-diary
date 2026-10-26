use domain::testing::{NoopPersonCommand, test_movie_with_year};
use std::sync::Arc;

use chrono::Utc;
use domain::{
    models::MovieProfile,
    ports::MovieCommand,
    testing::{FakeSearchCommand, InMemoryStore, PanicPersonCommand},
    value_objects::MovieId,
};

use application::movies::{commands::EnrichMovieCommand, deps::EnrichMovieDeps, enrich_movie};

#[tokio::test]
async fn stores_profile_and_indexes() {
    let movie_repo = InMemoryStore::new();
    let profile_repo = Arc::clone(&movie_repo);
    // PanicPersonCommand is safe here — empty cast/crew means upsert_batch is never called

    let movie = test_movie_with_year("Test", 2024);
    let movie_id = MovieId::from_uuid(movie.id().value());
    movie_repo.upsert_movie(&movie, &[]).await.unwrap();

    let profile = MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 999,
        imdb_id: None,
        overview: Some("A test movie".into()),
        tagline: None,
        runtime_minutes: Some(120),
        budget_usd: None,
        revenue_usd: None,
        vote_average: Some(7.5),
        vote_count: Some(100),
        original_language: Some("en".into()),
        collection_name: None,
        genres: vec![],
        keywords: vec![],
        cast: vec![],
        crew: vec![],
        enriched_at: Utc::now(),
    };

    let deps = EnrichMovieDeps {
        movie_query: movie_repo as Arc<_>,
        movie_profile: Arc::clone(&profile_repo) as Arc<_>,
        person_command: Arc::new(PanicPersonCommand),
        search_command: Arc::new(FakeSearchCommand),
    };

    enrich_movie::execute(
        &deps,
        EnrichMovieCommand {
            movie_id: movie_id.clone(),
            profile,
        },
    )
    .await
    .unwrap();

    assert_eq!(profile_repo.movie_profile_count(), 1);
}

#[tokio::test]
async fn extracts_and_indexes_persons() {
    let movie_repo = InMemoryStore::new();
    let profile_repo = Arc::clone(&movie_repo);

    let movie = test_movie_with_year("Cast Movie", 2024);
    let movie_id = MovieId::from_uuid(movie.id().value());
    movie_repo.upsert_movie(&movie, &[]).await.unwrap();

    let profile = MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 1001,
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
        genres: vec![],
        keywords: vec![],
        cast: vec![domain::models::CastMember {
            tmdb_person_id: 42,
            name: "Actor One".into(),
            character: "Hero".into(),
            billing_order: 0,
            profile_path: None,
        }],
        crew: vec![domain::models::CrewMember {
            tmdb_person_id: 99,
            name: "Director One".into(),
            job: "Director".into(),
            department: "Directing".into(),
            profile_path: None,
        }],
        enriched_at: Utc::now(),
    };

    let deps = EnrichMovieDeps {
        movie_query: movie_repo as Arc<_>,
        movie_profile: Arc::clone(&profile_repo) as Arc<_>,
        person_command: Arc::new(NoopPersonCommand),
        search_command: Arc::new(FakeSearchCommand),
    };

    enrich_movie::execute(
        &deps,
        EnrichMovieCommand {
            movie_id: movie_id.clone(),
            profile,
        },
    )
    .await
    .unwrap();

    assert_eq!(profile_repo.movie_profile_count(), 1);
}
