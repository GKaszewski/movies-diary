use std::sync::Arc;

use chrono::Utc;
use domain::{
    models::{Movie, MovieProfile},
    ports::MovieRepository,
    testing::{
        FakeSearchCommand, InMemoryMovieProfileRepository, InMemoryMovieRepository,
        PanicPersonCommand,
    },
    value_objects::{MovieId, MovieTitle, ReleaseYear},
};

use crate::movies::{commands::EnrichMovieCommand, enrich_movie};

#[tokio::test]
async fn stores_profile_and_indexes() {
    let movie_repo = InMemoryMovieRepository::new();
    let profile_repo = InMemoryMovieProfileRepository::new();
    let search_cmd: Arc<dyn domain::ports::SearchCommand> = Arc::new(FakeSearchCommand);
    // PanicPersonCommand is safe here — empty cast/crew means upsert_batch is never called
    let person_cmd: Arc<dyn domain::ports::PersonCommand> = Arc::new(PanicPersonCommand);

    let movie = Movie::new(
        None,
        MovieTitle::new("Test".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = MovieId::from_uuid(movie.id().value());
    movie_repo.upsert_movie(&movie).await.unwrap();

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

    enrich_movie::execute(
        &(movie_repo as Arc<_>),
        &(profile_repo.clone() as Arc<_>),
        &person_cmd,
        &search_cmd,
        EnrichMovieCommand {
            movie_id: movie_id.clone(),
            profile,
        },
    )
    .await
    .unwrap();

    assert_eq!(profile_repo.count(), 1);
}

struct NoopPersonCommand;

#[async_trait::async_trait]
impl domain::ports::PersonCommand for NoopPersonCommand {
    async fn upsert_batch(
        &self,
        _: &[domain::models::Person],
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
    async fn backfill_from_credits_batch(
        &self,
        _: u32,
    ) -> Result<(u64, bool), domain::errors::DomainError> {
        Ok((0, false))
    }
    async fn update_enrichment(
        &self,
        _: &domain::models::PersonId,
        _: &domain::models::PersonEnrichmentData,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
}

#[tokio::test]
async fn extracts_and_indexes_persons() {
    let movie_repo = InMemoryMovieRepository::new();
    let profile_repo = InMemoryMovieProfileRepository::new();
    let search_cmd: Arc<dyn domain::ports::SearchCommand> = Arc::new(FakeSearchCommand);
    let person_cmd: Arc<dyn domain::ports::PersonCommand> = Arc::new(NoopPersonCommand);

    let movie = Movie::new(
        None,
        MovieTitle::new("Cast Movie".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = MovieId::from_uuid(movie.id().value());
    movie_repo.upsert_movie(&movie).await.unwrap();

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

    enrich_movie::execute(
        &(movie_repo as Arc<_>),
        &(profile_repo.clone() as Arc<_>),
        &person_cmd,
        &search_cmd,
        EnrichMovieCommand {
            movie_id: movie_id.clone(),
            profile,
        },
    )
    .await
    .unwrap();

    assert_eq!(profile_repo.count(), 1);
}
