use application::worker::*;
use async_trait::async_trait;
use domain::testing::{InMemoryStore, NoopAckHandle};
use domain::value_objects::{ExternalMetadataId, MovieId};
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent},
};
use domain::{
    events::EventEnvelope,
    ports::{EventConsumer, EventHandler},
};
use futures::{StreamExt as _, stream, stream::BoxStream};
use std::sync::{Arc, Mutex};

struct TrackingAck {
    acked: Arc<Mutex<bool>>,
    nacked: Arc<Mutex<bool>>,
}

#[async_trait]
impl AckHandle for TrackingAck {
    async fn ack(&self) -> Result<(), DomainError> {
        *self.acked.lock().unwrap() = true;
        Ok(())
    }
    async fn nack(&self) -> Result<(), DomainError> {
        *self.nacked.lock().unwrap() = true;
        Ok(())
    }
    async fn defer(&self, _: std::time::Duration) -> Result<(), DomainError> {
        Ok(())
    }
}

struct VecConsumer {
    events: Vec<DomainEvent>,
}

impl EventConsumer for VecConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let envelopes: Vec<Result<EventEnvelope, DomainError>> = self
            .events
            .iter()
            .cloned()
            .map(|e| Ok(EventEnvelope::new(None, e, Box::new(NoopAckHandle))))
            .collect();
        Box::pin(stream::iter(envelopes))
    }
}

struct RecordingHandler {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl EventHandler for RecordingHandler {
    fn name(&self) -> &'static str {
        "recording"
    }
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let label = match event {
            DomainEvent::MovieDiscovered { .. } => "movie_discovered",
            DomainEvent::ReviewLogged { .. } => "review_logged",
            DomainEvent::ReviewUpdated { .. } => "review_updated",
            DomainEvent::ReviewDeleted { .. } => "review_deleted",
            DomainEvent::MovieDeleted { .. } => "movie_deleted",
            DomainEvent::UserUpdated { .. } => "user_updated",
            DomainEvent::MovieEnrichmentRequested { .. } => "movie_enrichment_requested",
            DomainEvent::ImageStored { .. } => "image_stored",
            DomainEvent::WatchlistEntryAdded { .. } | DomainEvent::WatchlistEntryRemoved { .. } => {
                "watchlist"
            }
            DomainEvent::FollowRequested { .. } => "follow_requested",
            DomainEvent::FollowAccepted { .. } => "follow_accepted",
            DomainEvent::FollowRejected { .. } => "follow_rejected",
            DomainEvent::Unfollowed { .. } => "unfollowed",
            DomainEvent::FollowerRemoved { .. } => "follower_removed",
            DomainEvent::ActorBlocked { .. } => "actor_blocked",
            DomainEvent::ActorUnblocked { .. } => "actor_unblocked",
            DomainEvent::BackfillFollower { .. } => "backfill_follower",
            DomainEvent::FederationDeliveryRequested { .. } => "federation_delivery",
            DomainEvent::WatchEventIngested { .. } => "watch_event_ingested",
            DomainEvent::WrapUpRequested { .. } => "wrapup_requested",
            DomainEvent::WrapUpCompleted { .. } => "wrapup_completed",
            DomainEvent::SearchReindexRequested => "search_reindex",
            DomainEvent::PosterSynced { .. } => "poster_synced",
            DomainEvent::GoalCreated { .. }
            | DomainEvent::GoalUpdated { .. }
            | DomainEvent::GoalDeleted { .. } => "goal",
            DomainEvent::PersonEnrichmentRequested { .. } => "person_enrichment_requested",
            DomainEvent::UserDeleted { .. } | DomainEvent::UserAccountMoved { .. } => {
                "user_lifecycle"
            }
        };
        self.calls.lock().unwrap().push(label);
        Ok(())
    }
}

fn movie_discovered() -> DomainEvent {
    DomainEvent::MovieDiscovered {
        movie_id: MovieId::generate(),
        external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
    }
}

#[tokio::test]
async fn dispatches_to_all_handlers() {
    let calls = Arc::new(Mutex::new(vec![]));
    let consumer = VecConsumer {
        events: vec![movie_discovered()],
    };
    let handler = RecordingHandler {
        calls: Arc::clone(&calls),
    };

    WorkerService::new(
        Arc::new(consumer),
        vec![Arc::new(handler)],
        InMemoryStore::new(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    assert_eq!(*calls.lock().unwrap(), vec!["movie_discovered"]);
}

#[tokio::test]
async fn acks_when_all_handlers_succeed() {
    let acked = Arc::new(Mutex::new(false));
    let nacked = Arc::new(Mutex::new(false));

    struct TrackingConsumer {
        event: DomainEvent,
        acked: Arc<Mutex<bool>>,
        nacked: Arc<Mutex<bool>>,
    }

    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                None,
                self.event.clone(),
                Box::new(TrackingAck {
                    acked: Arc::clone(&self.acked),
                    nacked: Arc::clone(&self.nacked),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }

    let consumer = TrackingConsumer {
        event: movie_discovered(),
        acked: Arc::clone(&acked),
        nacked: Arc::clone(&nacked),
    };

    WorkerService::new(Arc::new(consumer), vec![], InMemoryStore::new())
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert!(*acked.lock().unwrap());
    assert!(!*nacked.lock().unwrap());
}

#[tokio::test]
async fn nacks_on_transient_handler_failure() {
    let acked = Arc::new(Mutex::new(false));
    let nacked = Arc::new(Mutex::new(false));

    struct TrackingConsumer {
        event: DomainEvent,
        acked: Arc<Mutex<bool>>,
        nacked: Arc<Mutex<bool>>,
    }
    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                None,
                self.event.clone(),
                Box::new(TrackingAck {
                    acked: Arc::clone(&self.acked),
                    nacked: Arc::clone(&self.nacked),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }
    struct TransientHandler;
    #[async_trait]
    impl EventHandler for TransientHandler {
        fn name(&self) -> &'static str {
            "transient"
        }
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Err(DomainError::InfrastructureError("network timeout".into()))
        }
    }

    WorkerService::new(
        Arc::new(TrackingConsumer {
            event: movie_discovered(),
            acked: Arc::clone(&acked),
            nacked: Arc::clone(&nacked),
        }),
        vec![Arc::new(TransientHandler)],
        InMemoryStore::new(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    assert!(!*acked.lock().unwrap(), "should not ack on transient error");
    assert!(*nacked.lock().unwrap(), "should nack on transient error");
}

#[tokio::test]
async fn acks_on_permanent_handler_failure() {
    let acked = Arc::new(Mutex::new(false));
    let nacked = Arc::new(Mutex::new(false));

    struct TrackingConsumer {
        event: DomainEvent,
        acked: Arc<Mutex<bool>>,
        nacked: Arc<Mutex<bool>>,
    }
    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                None,
                self.event.clone(),
                Box::new(TrackingAck {
                    acked: Arc::clone(&self.acked),
                    nacked: Arc::clone(&self.nacked),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }
    struct PermanentHandler;
    #[async_trait]
    impl EventHandler for PermanentHandler {
        fn name(&self) -> &'static str {
            "permanent"
        }
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Err(DomainError::NotFound("movie not on tmdb".into()))
        }
    }

    WorkerService::new(
        Arc::new(TrackingConsumer {
            event: movie_discovered(),
            acked: Arc::clone(&acked),
            nacked: Arc::clone(&nacked),
        }),
        vec![Arc::new(PermanentHandler)],
        InMemoryStore::new(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    assert!(*acked.lock().unwrap(), "should ack on permanent error");
    assert!(
        !*nacked.lock().unwrap(),
        "should not nack on permanent error"
    );
}

#[tokio::test]
async fn nacks_if_any_handler_is_transient() {
    let acked = Arc::new(Mutex::new(false));
    let nacked = Arc::new(Mutex::new(false));

    struct TrackingConsumer {
        event: DomainEvent,
        acked: Arc<Mutex<bool>>,
        nacked: Arc<Mutex<bool>>,
    }
    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                None,
                self.event.clone(),
                Box::new(TrackingAck {
                    acked: Arc::clone(&self.acked),
                    nacked: Arc::clone(&self.nacked),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }
    struct OkHandler;
    #[async_trait]
    impl EventHandler for OkHandler {
        fn name(&self) -> &'static str {
            "ok"
        }
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Ok(())
        }
    }
    struct TransientHandler;
    #[async_trait]
    impl EventHandler for TransientHandler {
        fn name(&self) -> &'static str {
            "transient"
        }
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Err(DomainError::InfrastructureError("db gone".into()))
        }
    }

    WorkerService::new(
        Arc::new(TrackingConsumer {
            event: movie_discovered(),
            acked: Arc::clone(&acked),
            nacked: Arc::clone(&nacked),
        }),
        vec![Arc::new(OkHandler), Arc::new(TransientHandler)],
        InMemoryStore::new(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    assert!(
        !*acked.lock().unwrap(),
        "should not ack when any handler is transient"
    );
    assert!(
        *nacked.lock().unwrap(),
        "should nack when any handler is transient"
    );
}

// ── idempotent dispatch ───────────────────────────────────────────────────────

struct CountingHandler {
    name: &'static str,
    calls: Arc<Mutex<u32>>,
    fail_transient_times: Mutex<u32>,
}

impl CountingHandler {
    fn new(name: &'static str, fail_transient_times: u32) -> (Arc<Self>, Arc<Mutex<u32>>) {
        let calls = Arc::new(Mutex::new(0));
        (
            Arc::new(Self {
                name,
                calls: Arc::clone(&calls),
                fail_transient_times: Mutex::new(fail_transient_times),
            }),
            calls,
        )
    }
}

#[async_trait]
impl EventHandler for CountingHandler {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
        *self.calls.lock().unwrap() += 1;
        let mut left = self.fail_transient_times.lock().unwrap();
        if *left > 0 {
            *left -= 1;
            return Err(DomainError::InfrastructureError("flaky".into()));
        }
        Ok(())
    }
}

struct RedeliveringConsumer {
    id: Option<domain::events::EventId>,
    event: DomainEvent,
    times: usize,
    nacks: Arc<Mutex<u32>>,
    defers: Arc<Mutex<Vec<std::time::Duration>>>,
}

struct CountNack(Arc<Mutex<u32>>, Arc<Mutex<Vec<std::time::Duration>>>);

#[async_trait]
impl AckHandle for CountNack {
    async fn ack(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn nack(&self) -> Result<(), DomainError> {
        *self.0.lock().unwrap() += 1;
        Ok(())
    }
    async fn defer(&self, delay: std::time::Duration) -> Result<(), DomainError> {
        self.1.lock().unwrap().push(delay);
        Ok(())
    }
}

impl EventConsumer for RedeliveringConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let nacks = Arc::clone(&self.nacks);
        let defers = Arc::clone(&self.defers);
        let id = self.id;
        let event = self.event.clone();
        Box::pin(stream::iter(0..self.times).then(move |_| {
            let envelope = EventEnvelope::new(
                id,
                event.clone(),
                Box::new(CountNack(Arc::clone(&nacks), Arc::clone(&defers))),
            );
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                Ok(envelope)
            }
        }))
    }
}

#[tokio::test]
async fn a_redelivered_event_skips_handlers_that_already_succeeded() {
    let (first, first_calls) = CountingHandler::new("first", 0);
    let (second, second_calls) = CountingHandler::new("second", 1);
    let nacks = Arc::new(Mutex::new(0));
    let consumer = RedeliveringConsumer {
        id: Some(domain::events::EventId::new()),
        event: movie_discovered(),
        times: 2,
        nacks: Arc::clone(&nacks),
        defers: Default::default(),
    };

    WorkerService::new(
        Arc::new(consumer),
        vec![first, second],
        InMemoryStore::new(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    assert_eq!(*nacks.lock().unwrap(), 1, "only the first delivery nacks");
    assert_eq!(
        *first_calls.lock().unwrap(),
        1,
        "first ran once, skipped on redelivery"
    );
    assert_eq!(
        *second_calls.lock().unwrap(),
        2,
        "second retried and succeeded"
    );
}

#[tokio::test]
async fn events_without_id_run_every_time() {
    let (handler, calls) = CountingHandler::new("only", 0);
    let consumer = RedeliveringConsumer {
        id: None,
        event: movie_discovered(),
        times: 2,
        nacks: Arc::new(Mutex::new(0)),
        defers: Default::default(),
    };

    WorkerService::new(Arc::new(consumer), vec![handler], InMemoryStore::new())
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert_eq!(*calls.lock().unwrap(), 2);
}

#[tokio::test]
async fn duplicate_handler_names_are_rejected() {
    let (a, _) = CountingHandler::new("same", 0);
    let (b, _) = CountingHandler::new("same", 0);
    let result = WorkerService::new(
        Arc::new(VecConsumer { events: vec![] }),
        vec![a, b],
        InMemoryStore::new(),
    );
    let err = result.err().expect("duplicate names must be rejected");
    assert!(err.to_string().contains("same"), "{err}");
}

#[tokio::test]
async fn a_handler_already_running_elsewhere_is_not_run_concurrently() {
    use domain::ports::{Begin, ProcessedEventLedger};
    let (handler, calls) = CountingHandler::new("slow", 0);
    let store = InMemoryStore::new();
    let id = domain::events::EventId::new();
    assert!(matches!(
        store.begin(&id, "slow").await.unwrap(),
        Begin::Run(_)
    ));
    let nacks = Arc::new(Mutex::new(0));
    let defers: Arc<Mutex<Vec<std::time::Duration>>> = Default::default();
    let consumer = RedeliveringConsumer {
        id: Some(id),
        event: movie_discovered(),
        times: 1,
        nacks: Arc::clone(&nacks),
        defers: Arc::clone(&defers),
    };

    WorkerService::new(Arc::new(consumer), vec![handler], store)
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert_eq!(
        *calls.lock().unwrap(),
        0,
        "must not run beside the claim holder"
    );
    assert_eq!(
        *nacks.lock().unwrap(),
        0,
        "a blocked delivery is not a failure"
    );
    assert_eq!(defers.lock().unwrap().len(), 1, "delivery is retried later");
}

#[tokio::test]
async fn a_delivery_blocked_by_a_live_claim_is_deferred_without_consuming_an_attempt() {
    use domain::ports::{Begin, HANDLER_LEASE_SECONDS, ProcessedEventLedger};
    let (handler, calls) = CountingHandler::new("slow", 0);
    let store = InMemoryStore::new();
    let id = domain::events::EventId::new();
    assert!(matches!(
        store.begin(&id, "slow").await.unwrap(),
        Begin::Run(_)
    ));
    let nacks = Arc::new(Mutex::new(0));
    let defers: Arc<Mutex<Vec<std::time::Duration>>> = Default::default();
    let consumer = RedeliveringConsumer {
        id: Some(id),
        event: movie_discovered(),
        times: 1,
        nacks: Arc::clone(&nacks),
        defers: Arc::clone(&defers),
    };

    WorkerService::new(Arc::new(consumer), vec![handler], store)
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert_eq!(*calls.lock().unwrap(), 0);
    assert_eq!(*nacks.lock().unwrap(), 0, "a nack would consume an attempt");
    let defers = defers.lock().unwrap();
    assert_eq!(defers.len(), 1);
    let lease = std::time::Duration::from_secs(HANDLER_LEASE_SECONDS as u64);
    assert!(
        defers[0] > lease - std::time::Duration::from_secs(5),
        "{:?}",
        defers[0]
    );
    assert!(
        defers[0] <= lease + std::time::Duration::from_secs(6),
        "{:?}",
        defers[0]
    );
}

#[tokio::test]
async fn a_lapsed_owner_cannot_release_a_claim_taken_over_by_another() {
    use domain::ports::{Begin, HANDLER_LEASE_SECONDS, ProcessedEventLedger};
    let store = InMemoryStore::new();
    let id = domain::events::EventId::new();
    store.seed_processed_event(
        &id,
        "h",
        false,
        chrono::Utc::now().naive_utc() - chrono::Duration::seconds(HANDLER_LEASE_SECONDS + 1),
    );
    let Begin::Run(lapsed) = store.begin(&id, "h").await.unwrap() else {
        panic!("stale claim should be taken over");
    };
    let original = domain::ports::ClaimToken::new("2020-01-01T00:00:00.000Z".into());
    assert_ne!(original, lapsed);

    store.release(&id, "h", &original).await.unwrap();
    store.complete(&id, "h", &original).await.unwrap();

    assert!(
        matches!(
            store.begin(&id, "h").await.unwrap(),
            Begin::InProgressElsewhere { .. }
        ),
        "the new owner's claim must survive the old owner's release and complete"
    );
}

#[tokio::test]
async fn a_stale_in_progress_claim_is_taken_over() {
    use domain::ports::HANDLER_LEASE_SECONDS;
    let (handler, calls) = CountingHandler::new("crashed", 0);
    let store = InMemoryStore::new();
    let id = domain::events::EventId::new();
    store.seed_processed_event(
        &id,
        "crashed",
        false,
        chrono::Utc::now().naive_utc() - chrono::Duration::seconds(HANDLER_LEASE_SECONDS + 1),
    );
    let nacks = Arc::new(Mutex::new(0));
    let consumer = RedeliveringConsumer {
        id: Some(id),
        event: movie_discovered(),
        times: 1,
        nacks: Arc::clone(&nacks),
        defers: Default::default(),
    };

    WorkerService::new(Arc::new(consumer), vec![handler], store)
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(*nacks.lock().unwrap(), 0);
}

#[tokio::test]
async fn a_failed_handler_releases_its_claim_so_redelivery_retries() {
    let (handler, calls) = CountingHandler::new("flaky", 1);
    let store = InMemoryStore::new();
    let nacks = Arc::new(Mutex::new(0));
    let consumer = RedeliveringConsumer {
        id: Some(domain::events::EventId::new()),
        event: movie_discovered(),
        times: 3,
        nacks: Arc::clone(&nacks),
        defers: Default::default(),
    };

    WorkerService::new(Arc::new(consumer), vec![handler], store.clone())
        .unwrap()
        .run(tokio::sync::watch::channel(false).1)
        .await;

    assert_eq!(*calls.lock().unwrap(), 2);
    assert_eq!(*nacks.lock().unwrap(), 1);
}
