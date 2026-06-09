use std::sync::Arc;

use uuid::Uuid;

use domain::{
    errors::DomainError,
    models::Movie,
    ports::{MetadataClient, MovieRepository},
    testing::InMemoryMovieRepository,
    value_objects::{ExternalMetadataId, MovieTitle, PosterUrl, ReleaseYear},
};

use crate::{
    diary::commands::SyncPosterCommand, movies::sync_poster, test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn fails_when_movie_not_found() {
    let ctx = TestContextBuilder::new().build();

    let result = sync_poster::execute(
        &ctx,
        SyncPosterCommand {
            movie_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn fails_when_no_external_id() {
    let movies = InMemoryMovieRepository::new();
    let movie = Movie::new(
        None,
        MovieTitle::new("Test".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .build();

    let result = sync_poster::execute(&ctx, SyncPosterCommand { movie_id }).await;

    assert!(result.is_err());
}

struct FakeMetaWithPoster;

#[async_trait::async_trait]
impl MetadataClient for FakeMetaWithPoster {
    async fn fetch_movie_metadata(
        &self,
        _: &domain::ports::MetadataSearchCriteria,
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
    let movies = InMemoryMovieRepository::new();
    let ext_id = ExternalMetadataId::new("tmdb:999".into()).unwrap();
    let movie = Movie::new(
        Some(ext_id),
        MovieTitle::new("Poster Movie".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = movie.id().value();
    movies.upsert_movie(&movie).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_metadata_client(Arc::new(FakeMetaWithPoster) as _)
        .build();

    sync_poster::execute(&ctx, SyncPosterCommand { movie_id })
        .await
        .unwrap();

    let updated = movies
        .get_movie_by_id(&domain::value_objects::MovieId::from_uuid(movie_id))
        .await
        .unwrap()
        .unwrap();
    assert!(updated.poster_path().is_some());
}
