use super::*;
use async_trait::async_trait;
use domain::value_objects::{ExternalMetadataId, MovieId};
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent},
};
use futures::{stream, stream::BoxStream};
use std::sync::{Arc, Mutex};

struct NoopAck;

#[async_trait]
impl AckHandle for NoopAck {
    async fn ack(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn nack(&self) -> Result<(), DomainError> {
        Ok(())
    }
}

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
            .map(|e| Ok(EventEnvelope::new(e, Box::new(NoopAck))))
            .collect();
        Box::pin(stream::iter(envelopes))
    }
}

struct RecordingHandler {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl EventHandler for RecordingHandler {
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

    WorkerService::new(Arc::new(consumer), vec![Arc::new(handler)])
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

    WorkerService::new(Arc::new(consumer), vec![])
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
    )
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
    )
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
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Ok(())
        }
    }
    struct TransientHandler;
    #[async_trait]
    impl EventHandler for TransientHandler {
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
    )
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
