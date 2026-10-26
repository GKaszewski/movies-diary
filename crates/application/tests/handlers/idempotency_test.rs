use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        EntityType, ExternalPersonId, IndexableDocument, Movie, MovieProfile, Person,
        PersonCredits, PersonEnrichmentData, PersonId,
    },
    ports::{
        EventHandler, MetadataClient, MovieCommand, MovieEnrichmentClient, MovieProfileRepository,
        PersonCommand, PersonEnrichmentClient, PersonQuery, SearchCommand,
    },
    testing::{
        FakePosterFetcher, InMemoryStore, NoopEventPublisher, NoopObjectStorage, NoopPersonCommand,
        test_movie_with_external_id,
    },
    value_objects::{ExternalMetadataId, MovieId, PosterPath, PosterUrl},
};

use application::movies::{
    deps::ReindexSearchDeps, discovery_indexer::MovieDiscoveryIndexer,
    event_handler::MovieEnrichmentHandler, poster_sync_handler::PosterSyncHandler,
    reindex_search::SearchReindexHandler, search_cleanup::SearchCleanupHandler,
};
use application::person::event_handler::PersonEnrichmentHandler;

// ── fakes ───────────────────────────────────────────────────────────────────

#[derive(Default)]
struct RecordingSearch {
    docs: Mutex<HashMap<(String, String), u32>>,
}

impl RecordingSearch {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn snapshot(&self) -> Vec<(String, String)> {
        let mut keys: Vec<_> = self.docs.lock().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }
}

fn doc_key(doc: &IndexableDocument) -> (String, String) {
    match doc {
        IndexableDocument::Movie { id, .. } => ("movie".into(), id.value().to_string()),
        IndexableDocument::Person { id, .. } => ("person".into(), id.value().to_string()),
    }
}

#[async_trait]
impl SearchCommand for RecordingSearch {
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError> {
        self.docs.lock().unwrap().insert(doc_key(&doc), 1);
        Ok(())
    }
    async fn remove(&self, entity_type: EntityType, id: &str) -> Result<(), DomainError> {
        let kind = match entity_type {
            EntityType::Movie => "movie",
            EntityType::Person => "person",
        };
        self.docs
            .lock()
            .unwrap()
            .remove(&(kind.to_string(), id.to_string()));
        Ok(())
    }
}

struct FlakySearch {
    inner: Arc<RecordingSearch>,
    failed_once: AtomicUsize,
}

#[async_trait]
impl SearchCommand for FlakySearch {
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError> {
        if self.failed_once.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(DomainError::InfrastructureError("index down".into()));
        }
        self.inner.index(doc).await
    }
    async fn remove(&self, t: EntityType, id: &str) -> Result<(), DomainError> {
        self.inner.remove(t, id).await
    }
}

fn person(tmdb: u64, name: &str) -> Person {
    let ext = ExternalPersonId::new(format!("tmdb:{tmdb}"));
    Person::basic(PersonId::from_external(&ext), ext, name.into(), None, None)
}

struct FakePersons {
    person: Mutex<Person>,
    enrich_writes: AtomicUsize,
}

impl FakePersons {
    fn new(p: Person) -> Arc<Self> {
        Arc::new(Self {
            person: Mutex::new(p),
            enrich_writes: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl PersonQuery for FakePersons {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError> {
        let p = self.person.lock().unwrap().clone();
        Ok((p.id() == id).then_some(p))
    }
    async fn get_by_external_id(
        &self,
        _: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        Ok(None)
    }
    async fn get_credits(&self, _: &PersonId) -> Result<PersonCredits, DomainError> {
        unimplemented!()
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        Ok(vec![self.person.lock().unwrap().id().clone()])
    }
    async fn list_page(&self, _limit: u32, offset: u32) -> Result<Vec<Person>, DomainError> {
        if offset == 0 {
            Ok(vec![self.person.lock().unwrap().clone()])
        } else {
            Ok(vec![])
        }
    }
}

#[async_trait]
impl PersonCommand for FakePersons {
    async fn upsert_batch(&self, _: &[Person]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn backfill_from_credits_batch(&self, _: u32) -> Result<(u64, bool), DomainError> {
        Ok((0, false))
    }
    async fn update_enrichment(
        &self,
        id: &PersonId,
        data: &PersonEnrichmentData,
    ) -> Result<(), DomainError> {
        self.enrich_writes.fetch_add(1, Ordering::SeqCst);
        let mut p = self.person.lock().unwrap();
        assert_eq!(p.id(), id);
        *p = Person::new(
            p.id().clone(),
            p.external_id().clone(),
            p.name().to_string(),
            p.known_for_department().map(str::to_string),
            p.profile_path().map(str::to_string),
            data.biography.clone(),
            data.birthday,
            data.deathday,
            data.place_of_birth.clone(),
            data.also_known_as.clone(),
            data.homepage.clone(),
            data.imdb_id.clone(),
            Some(Utc::now()),
        );
        Ok(())
    }
}

struct CountingPersonClient(AtomicUsize);

#[async_trait]
impl PersonEnrichmentClient for CountingPersonClient {
    async fn fetch_details(&self, _: &str) -> Result<PersonEnrichmentData, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(PersonEnrichmentData {
            biography: Some("bio".into()),
            birthday: None,
            deathday: None,
            place_of_birth: None,
            also_known_as: vec![],
            homepage: None,
            imdb_id: None,
        })
    }
}

fn profile_for(movie_id: &MovieId) -> MovieProfile {
    MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 42,
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
    }
}

struct CountingMovieClient(AtomicUsize);

#[async_trait]
impl MovieEnrichmentClient for CountingMovieClient {
    async fn fetch_profile(&self, movie_id: MovieId, _: &str) -> Result<MovieProfile, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(profile_for(&movie_id))
    }
}

struct NoImages;

#[async_trait]
impl domain::ports::ImageFetcher for NoImages {
    async fn fetch_image(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::NotFound("none".into()))
    }
}

struct KeyStorage;

#[async_trait]
impl domain::ports::ObjectStorage for KeyStorage {
    async fn store(&self, key: &str, _: &[u8]) -> Result<String, DomainError> {
        Ok(key.to_string())
    }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        Ok(vec![])
    }
    async fn get_stream(
        &self,
        _: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn exists(&self, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
}

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
            PosterUrl::new("https://example.com/p.jpg".into()).unwrap(),
        ))
    }
}

async fn seeded_movie(store: &Arc<InMemoryStore>) -> (Movie, ExternalMetadataId) {
    let ext = ExternalMetadataId::new("tmdb:42".into()).unwrap();
    let movie = test_movie_with_external_id("Heat", 1995, ext.clone());
    store.upsert_movie(&movie, &[]).await.unwrap();
    (movie, ext)
}

// ── person_enrichment ───────────────────────────────────────────────────────

#[tokio::test]
async fn person_enrichment_is_idempotent_when_run_twice() {
    let p = person(7, "Al Pacino");
    let persons = FakePersons::new(p.clone());
    let client = Arc::new(CountingPersonClient(AtomicUsize::new(0)));
    let handler = PersonEnrichmentHandler::new(
        persons.clone(),
        Some(client.clone() as Arc<dyn PersonEnrichmentClient>),
        persons.clone(),
    );
    let event = DomainEvent::PersonEnrichmentRequested {
        person_id: p.id().clone(),
        external_person_id: p.external_id().clone(),
    };

    handler.handle(&event).await.unwrap();
    handler.handle(&event).await.unwrap();

    assert_eq!(client.0.load(Ordering::SeqCst), 1);
    assert_eq!(persons.enrich_writes.load(Ordering::SeqCst), 1);
}

// ── movie_enrichment ────────────────────────────────────────────────────────

fn enrichment_handler(
    store: &Arc<InMemoryStore>,
    client: Arc<CountingMovieClient>,
    search: Arc<dyn SearchCommand>,
) -> MovieEnrichmentHandler {
    MovieEnrichmentHandler::new(
        client,
        Arc::clone(store) as _,
        Arc::clone(store) as _,
        Arc::new(NoopPersonCommand),
        search,
        Arc::new(NoopObjectStorage),
        Arc::new(NoImages),
    )
}

#[tokio::test]
async fn movie_enrichment_is_idempotent_when_run_twice() {
    let store = InMemoryStore::new();
    let (movie, ext) = seeded_movie(&store).await;
    let search = RecordingSearch::new();
    let client = Arc::new(CountingMovieClient(AtomicUsize::new(0)));
    let handler = enrichment_handler(&store, client.clone(), search.clone());
    let event = DomainEvent::MovieEnrichmentRequested {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };

    handler.handle(&event).await.unwrap();
    let after_first = search.snapshot();
    handler.handle(&event).await.unwrap();

    assert_eq!(search.snapshot(), after_first);
    assert_eq!(client.0.load(Ordering::SeqCst), 1, "no second remote fetch");
}

#[tokio::test]
async fn a_movie_enrichment_that_failed_midway_is_redone_on_redelivery() {
    let store = InMemoryStore::new();
    let (movie, ext) = seeded_movie(&store).await;
    let search = RecordingSearch::new();
    let flaky = Arc::new(FlakySearch {
        inner: search.clone(),
        failed_once: AtomicUsize::new(0),
    });
    let client = Arc::new(CountingMovieClient(AtomicUsize::new(0)));
    let handler = enrichment_handler(&store, client, flaky);
    let event = DomainEvent::MovieEnrichmentRequested {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };

    assert!(handler.handle(&event).await.is_err());
    assert!(store.get_by_movie_id(movie.id()).await.unwrap().is_none());

    handler.handle(&event).await.unwrap();
    assert_eq!(search.snapshot().len(), 1);
    assert!(store.get_by_movie_id(movie.id()).await.unwrap().is_some());
}

// ── search_cleanup ──────────────────────────────────────────────────────────

#[tokio::test]
async fn search_cleanup_is_idempotent_when_run_twice() {
    let orphan = person(9, "Orphan");
    let persons = FakePersons::new(orphan.clone());
    let search = RecordingSearch::new();
    let movie_id = MovieId::from_uuid(uuid::Uuid::new_v4());
    search
        .index(IndexableDocument::Person {
            id: orphan.id().clone(),
            person: Box::new(orphan.clone()),
        })
        .await
        .unwrap();
    let handler = SearchCleanupHandler::new(search.clone(), persons);
    let event = DomainEvent::MovieDeleted {
        movie_id,
        poster_path: None,
    };

    handler.handle(&event).await.unwrap();
    let after_first = search.snapshot();
    handler.handle(&event).await.unwrap();

    assert!(after_first.is_empty());
    assert_eq!(search.snapshot(), after_first);
}

// ── movie_discovery_indexer ─────────────────────────────────────────────────

#[tokio::test]
async fn movie_discovery_indexer_is_idempotent_when_run_twice() {
    let store = InMemoryStore::new();
    let (movie, ext) = seeded_movie(&store).await;
    let search = RecordingSearch::new();
    let handler = MovieDiscoveryIndexer::new(store.clone(), search.clone());
    let event = DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };

    handler.handle(&event).await.unwrap();
    let after_first = search.snapshot();
    handler.handle(&event).await.unwrap();

    assert_eq!(after_first.len(), 1);
    assert_eq!(search.snapshot(), after_first);
}

// ── search_reindex ──────────────────────────────────────────────────────────

#[tokio::test]
async fn search_reindex_is_idempotent_when_run_twice() {
    let store = InMemoryStore::new();
    let (movie, _) = seeded_movie(&store).await;
    let persons = FakePersons::new(person(3, "Someone"));
    let search = RecordingSearch::new();
    let handler = SearchReindexHandler::new(ReindexSearchDeps {
        movie_query: store.clone(),
        movie_profile: store.clone(),
        search_command: search.clone(),
        person_command: persons.clone(),
        person_query: persons,
    });

    handler
        .handle(&DomainEvent::SearchReindexRequested)
        .await
        .unwrap();
    let after_first = search.snapshot();
    handler
        .handle(&DomainEvent::SearchReindexRequested)
        .await
        .unwrap();

    assert!(
        after_first.contains(&("movie".to_string(), movie.id().value().to_string())),
        "the movie doc is indexed"
    );
    assert!(after_first.iter().any(|(k, _)| k == "person"));
    assert_eq!(search.snapshot(), after_first);
}

// ── poster_sync ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn poster_sync_is_idempotent_when_run_twice() {
    let store = InMemoryStore::new();
    let (movie, ext) = seeded_movie(&store).await;
    let publisher = NoopEventPublisher::new();
    let handler = PosterSyncHandler::new(
        store.clone(),
        store.clone(),
        Arc::new(MetadataWithPoster),
        Arc::new(FakePosterFetcher),
        Arc::new(KeyStorage),
        publisher.clone(),
        0,
    );
    let event = DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };

    handler.handle(&event).await.unwrap();
    handler.handle(&event).await.unwrap();

    let synced = store
        .recorded_events()
        .iter()
        .filter(|e| matches!(e, DomainEvent::PosterSynced { .. }))
        .count();
    assert_eq!(synced, 1, "PosterSynced fans out to federation: emit once");
    assert_eq!(publisher.published().len(), 1, "one ImageStored, not two");
}

#[tokio::test]
async fn poster_sync_skips_converted_poster_keys_but_not_lookalike_paths() {
    let store = InMemoryStore::new();
    let (mut movie, ext) = seeded_movie(&store).await;
    let id = movie.id().value().to_string();
    let publisher = NoopEventPublisher::new();
    let handler = PosterSyncHandler::new(
        store.clone(),
        store.clone(),
        Arc::new(MetadataWithPoster),
        Arc::new(FakePosterFetcher),
        Arc::new(KeyStorage),
        publisher.clone(),
        0,
    );
    let event = DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: ext,
    };

    movie.update_poster(PosterPath::new(format!("{id}.webp")).unwrap());
    store.upsert_movie(&movie, &[]).await.unwrap();
    handler.handle(&event).await.unwrap();
    assert!(publisher.published().is_empty());

    movie.update_poster(PosterPath::new(format!("https://cdn.example/{id}/p.jpg")).unwrap());
    store.upsert_movie(&movie, &[]).await.unwrap();
    handler.handle(&event).await.unwrap();
    assert_eq!(publisher.published().len(), 1);
}

#[tokio::test]
async fn enrichment_of_a_deleted_movie_stores_no_profile() {
    let store = InMemoryStore::new();
    let ghost = MovieId::from_uuid(uuid::Uuid::new_v4());
    let search = RecordingSearch::new();
    let client = Arc::new(CountingMovieClient(AtomicUsize::new(0)));
    let handler = enrichment_handler(&store, client, search.clone());
    let event = DomainEvent::MovieEnrichmentRequested {
        movie_id: ghost.clone(),
        external_metadata_id: ExternalMetadataId::new("tmdb:1".into()).unwrap(),
    };

    handler.handle(&event).await.unwrap();

    assert!(store.get_by_movie_id(&ghost).await.unwrap().is_none());
    assert!(search.snapshot().is_empty());
}
