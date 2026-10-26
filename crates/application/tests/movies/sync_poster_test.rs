use crate::support::sync_poster_command;
use domain::testing::{test_movie_with_external_id, test_movie_with_year};
use std::sync::Arc;

use uuid::Uuid;

use domain::{
    errors::DomainError,
    models::Movie,
    ports::{MetadataClient, MovieCommand, MovieQuery},
    testing::{FakeSearchCommand, InMemoryStore, NoopEventPublisher, NoopObjectStorage},
    value_objects::{ExternalMetadataId, PosterUrl},
};

use application::movies::{deps::SyncPosterDeps, sync_poster};

fn default_deps() -> SyncPosterDeps {
    SyncPosterDeps {
        movie_command: InMemoryStore::new(),
        movie_query: InMemoryStore::new(),
        movie_profile: InMemoryStore::new(),
        metadata: Arc::new(domain::testing::FakeMetadataClient),
        poster_fetcher: Arc::new(domain::testing::FakePosterFetcher),
        object_storage: Arc::new(NoopObjectStorage),
        event_publisher: NoopEventPublisher::new(),
        search_command: Arc::new(FakeSearchCommand),
    }
}

#[tokio::test]
async fn fails_when_movie_not_found() {
    let deps = default_deps();

    let result = sync_poster::execute(&deps, sync_poster_command(Uuid::new_v4())).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn fails_when_no_external_id() {
    let movies = InMemoryStore::new();
    let movie = test_movie_with_year("Test", 2024);
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let deps = SyncPosterDeps {
        movie_command: Arc::clone(&movies) as _,
        movie_query: Arc::clone(&movies) as _,
        ..default_deps()
    };

    let result = sync_poster::execute(&deps, sync_poster_command(movie_id)).await;

    assert!(result.is_err());
}

struct FakeMetaWithPoster;

#[async_trait::async_trait]
impl MetadataClient for FakeMetaWithPoster {
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

#[tokio::test]
async fn syncs_poster_for_movie_with_external_id() {
    let movies = InMemoryStore::new();
    let ext_id = ExternalMetadataId::new("tmdb:999".into()).unwrap();
    let movie = test_movie_with_external_id("Poster Movie", 2024, ext_id);
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let deps = SyncPosterDeps {
        movie_command: Arc::clone(&movies) as _,
        movie_query: Arc::clone(&movies) as _,
        metadata: Arc::new(FakeMetaWithPoster) as _,
        ..default_deps()
    };

    sync_poster::execute(&deps, sync_poster_command(movie_id))
        .await
        .unwrap();

    let updated = movies
        .get_movie_by_id(&domain::value_objects::MovieId::from_uuid(movie_id))
        .await
        .unwrap()
        .unwrap();
    assert!(updated.poster_path().is_some());
}
