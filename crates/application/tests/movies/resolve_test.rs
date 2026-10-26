use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{MetadataSearchCriteria, Movie},
    ports::{MetadataClient, MovieCommand},
    testing::{
        FakeMetadataClient, InMemoryStore, RefusingMovieCommand, test_movie_with_external_id,
        test_movie_with_year,
    },
    value_objects::{ExternalMetadataId, PosterUrl},
};

use application::diary::commands::MovieInput;
use application::movies::resolve::resolve_and_persist_movie;

struct MetadataWithExternalId;

#[async_trait]
impl MetadataClient for MetadataWithExternalId {
    async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> {
        Ok(test_movie_with_external_id(
            "Discovered Film",
            2024,
            ExternalMetadataId::new("tmdb:99999".into()).unwrap(),
        ))
    }

    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        Ok(None)
    }
}

fn manual_input(title: &str) -> MovieInput {
    MovieInput {
        movie_id: None,
        external_metadata_id: None,
        manual_title: Some(title.into()),
        manual_release_year: Some(2024),
        manual_director: None,
    }
}

#[tokio::test]
async fn a_new_movie_with_an_external_id_is_persisted_with_movie_discovered() {
    let store = InMemoryStore::new();

    let (movie, is_new) = resolve_and_persist_movie(
        &manual_input("Discovered Film"),
        store.as_ref(),
        store.as_ref(),
        &MetadataWithExternalId,
    )
    .await
    .unwrap();

    assert!(is_new);
    assert_eq!(store.movie_count(), 1);
    let recorded = store.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        &recorded[0],
        DomainEvent::MovieDiscovered { movie_id, .. } if movie_id == movie.id()
    ));
}

#[tokio::test]
async fn a_new_movie_without_an_external_id_records_no_event() {
    let store = InMemoryStore::new();

    let (_, is_new) = resolve_and_persist_movie(
        &manual_input("Home Movie"),
        store.as_ref(),
        store.as_ref(),
        &FakeMetadataClient,
    )
    .await
    .unwrap();

    assert!(is_new);
    assert_eq!(store.movie_count(), 1);
    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn an_existing_movie_is_not_rewritten_and_records_no_event() {
    let store = InMemoryStore::new();
    let movie = test_movie_with_year("Known", 2001);
    store.upsert_movie(&movie, &[]).await.unwrap();
    let input = MovieInput {
        movie_id: Some(movie.id().value()),
        ..manual_input("ignored")
    };

    let (resolved, is_new) = resolve_and_persist_movie(
        &input,
        store.as_ref(),
        store.as_ref(),
        &MetadataWithExternalId,
    )
    .await
    .unwrap();

    assert!(!is_new);
    assert_eq!(resolved.id(), movie.id());
    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn a_refused_upsert_propagates_and_records_nothing() {
    let store = InMemoryStore::new();

    let result = resolve_and_persist_movie(
        &manual_input("Discovered Film"),
        &RefusingMovieCommand,
        store.as_ref(),
        &MetadataWithExternalId,
    )
    .await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
    assert_eq!(store.movie_count(), 0);
    assert!(store.recorded_events().is_empty());
}
