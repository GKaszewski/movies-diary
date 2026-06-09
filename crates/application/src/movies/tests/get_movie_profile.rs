use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use domain::{
    models::{CastMember, CrewMember, MovieProfile},
    ports::MovieProfileRepository,
    testing::InMemoryMovieProfileRepository,
    value_objects::MovieId,
};

use crate::{
    movies::get_movie_profile::{self, GetMovieProfileQuery},
    test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn returns_none_when_no_profile() {
    let ctx = TestContextBuilder::new().build();

    let result = get_movie_profile::execute(
        &ctx,
        GetMovieProfileQuery {
            movie_id: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn returns_profile_with_cast_and_crew() {
    let profile_repo = InMemoryMovieProfileRepository::new();
    let movie_id = MovieId::generate();

    let profile = MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 42,
        imdb_id: Some("tt1234567".into()),
        overview: Some("A great movie".into()),
        tagline: None,
        runtime_minutes: Some(120),
        budget_usd: None,
        revenue_usd: None,
        vote_average: Some(8.0),
        vote_count: Some(500),
        original_language: Some("en".into()),
        collection_name: None,
        genres: vec![],
        keywords: vec![],
        cast: vec![CastMember {
            tmdb_person_id: 1,
            name: "Alice".into(),
            character: "Hero".into(),
            billing_order: 0,
            profile_path: None,
        }],
        crew: vec![CrewMember {
            tmdb_person_id: 2,
            name: "Bob".into(),
            job: "Director".into(),
            department: "Directing".into(),
            profile_path: None,
        }],
        enriched_at: Utc::now(),
    };
    profile_repo.upsert(&profile).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_movie_profiles(Arc::clone(&profile_repo) as _)
        .build();

    let result = get_movie_profile::execute(
        &ctx,
        GetMovieProfileQuery {
            movie_id: movie_id.value(),
        },
    )
    .await
    .unwrap();

    let res = result.expect("profile should be present");
    assert_eq!(res.cast.len(), 1);
    assert_eq!(res.cast[0].name, "Alice");
    assert_eq!(res.cast[0].character, "Hero");
    assert_eq!(res.crew.len(), 1);
    assert_eq!(res.crew[0].name, "Bob");
    assert_eq!(res.crew[0].job, "Director");
}
