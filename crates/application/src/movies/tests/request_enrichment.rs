use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    models::MovieProfile,
    ports::{MovieEnrichmentClient, MovieProfileRepository},
    testing::{FakeMovieEnrichmentClient, InMemoryMovieProfileRepository},
    value_objects::MovieId,
};

use crate::movies::request_enrichment;

#[tokio::test]
async fn returns_profile_when_none_cached() {
    let enrichment = FakeMovieEnrichmentClient;
    let profile_repo = InMemoryMovieProfileRepository::new();
    let movie_id = MovieId::generate();

    let result = request_enrichment::fetch_if_stale(
        &enrichment,
        &(profile_repo as Arc<_>),
        movie_id.clone(),
        "tmdb:12345",
    )
    .await
    .unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().movie_id, movie_id);
}

#[tokio::test]
async fn returns_none_when_profile_is_fresh() {
    let enrichment = FakeMovieEnrichmentClient;
    let profile_repo = InMemoryMovieProfileRepository::new();
    let movie_id = MovieId::generate();

    // Seed a fresh profile (enriched_at = now)
    let fresh_profile = MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 12345,
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
        cast: vec![],
        crew: vec![],
        enriched_at: Utc::now(),
    };
    profile_repo.upsert(&fresh_profile).await.unwrap();

    let result = request_enrichment::fetch_if_stale(
        &enrichment,
        &(Arc::clone(&profile_repo) as Arc<_>),
        movie_id,
        "tmdb:12345",
    )
    .await
    .unwrap();

    assert!(result.is_none(), "fresh profile should be skipped");
}

struct NotFoundEnrichmentClient;

#[async_trait]
impl MovieEnrichmentClient for NotFoundEnrichmentClient {
    async fn fetch_profile(
        &self,
        _movie_id: MovieId,
        _external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError> {
        Err(DomainError::NotFound("not found in TMDb".into()))
    }
}

#[tokio::test]
async fn returns_none_on_not_found_from_client() {
    let enrichment = NotFoundEnrichmentClient;
    let profile_repo = InMemoryMovieProfileRepository::new();
    let movie_id = MovieId::generate();

    let result = request_enrichment::fetch_if_stale(
        &enrichment,
        &(profile_repo as Arc<_>),
        movie_id,
        "tmdb:99999",
    )
    .await
    .unwrap();

    assert!(result.is_none(), "NotFound should return Ok(None), not Err");
}
