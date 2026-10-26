use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::Movie,
    ports::{EventHandler, MetadataClient, MovieCommand},
    testing::{
        FakePosterFetcher, InMemoryStore, NoopEventPublisher, NoopObjectStorage,
        RefusingMovieCommand, test_movie_with_external_id,
    },
    value_objects::{ExternalMetadataId, PosterUrl},
};

use application::movies::poster_sync_handler::PosterSyncHandler;

struct MetadataWithPoster;

#[async_trait]
impl MetadataClient for MetadataWithPoster {
    async fn fetch_movie_metadata(
        &self,
        _: &domain::models::MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        unimplemented!()
    }
    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        Ok(Some(
            PosterUrl::new("https://example.com/poster.jpg".into()).unwrap(),
        ))
    }
}

async fn seeded() -> (Arc<InMemoryStore>, Movie, DomainEvent) {
    let store = InMemoryStore::new();
    let ext = ExternalMetadataId::new("tmdb:42".into()).unwrap();
    let movie = test_movie_with_external_id("Heat", 1995, ext.clone());
    store.upsert_movie(&movie, &[]).await.unwrap();
    let event = DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };
    (store, movie, event)
}

fn handler(command: Arc<dyn MovieCommand>, store: &Arc<InMemoryStore>) -> PosterSyncHandler {
    PosterSyncHandler::new(
        command,
        Arc::clone(store) as _,
        Arc::new(MetadataWithPoster),
        Arc::new(FakePosterFetcher),
        Arc::new(NoopObjectStorage),
        NoopEventPublisher::new(),
        0,
    )
}

#[tokio::test]
async fn poster_synced_rides_the_movie_write() {
    let (store, movie, event) = seeded().await;

    handler(Arc::clone(&store) as _, &store)
        .handle(&event)
        .await
        .unwrap();

    let recorded = store.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        &recorded[0],
        DomainEvent::PosterSynced { movie_id } if movie_id == movie.id()
    ));
}

#[tokio::test]
async fn a_refused_upsert_propagates_and_records_nothing() {
    let (store, _movie, event) = seeded().await;

    let result = handler(Arc::new(RefusingMovieCommand), &store)
        .handle(&event)
        .await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
    assert!(store.recorded_events().is_empty());
}
