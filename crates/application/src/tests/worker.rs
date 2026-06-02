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
            DomainEvent::FollowAccepted { .. } => "follow_accepted",
            DomainEvent::BackfillFollower { .. } => "backfill_follower",
            DomainEvent::FederationDeliveryRequested { .. } => "federation_delivery",
            DomainEvent::WatchEventIngested { .. } => "watch_event_ingested",
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
        .run()
        .await;

    assert_eq!(*calls.lock().unwrap(), vec!["movie_discovered"]);
}

#[tokio::test]
async fn nacks_when_handler_fails() {
    let nack_called = Arc::new(Mutex::new(false));

    struct TrackingAck {
        nack_called: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl AckHandle for TrackingAck {
        async fn ack(&self) -> Result<(), DomainError> {
            Ok(())
        }
        async fn nack(&self) -> Result<(), DomainError> {
            *self.nack_called.lock().unwrap() = true;
            Ok(())
        }
    }

    struct TrackingConsumer {
        event: DomainEvent,
        nack_called: Arc<Mutex<bool>>,
    }

    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                self.event.clone(),
                Box::new(TrackingAck {
                    nack_called: Arc::clone(&self.nack_called),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }

    struct FailingHandler;

    #[async_trait]
    impl EventHandler for FailingHandler {
        async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Err(DomainError::InfrastructureError("boom".into()))
        }
    }

    let consumer = TrackingConsumer {
        event: movie_discovered(),
        nack_called: Arc::clone(&nack_called),
    };

    WorkerService::new(Arc::new(consumer), vec![Arc::new(FailingHandler)])
        .run()
        .await;

    assert!(*nack_called.lock().unwrap());
}

#[tokio::test]
async fn acks_when_all_handlers_succeed() {
    let ack_called = Arc::new(Mutex::new(false));

    struct TrackingAck {
        ack_called: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl AckHandle for TrackingAck {
        async fn ack(&self) -> Result<(), DomainError> {
            *self.ack_called.lock().unwrap() = true;
            Ok(())
        }
        async fn nack(&self) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct TrackingConsumer {
        event: DomainEvent,
        ack_called: Arc<Mutex<bool>>,
    }

    impl EventConsumer for TrackingConsumer {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            let envelope = EventEnvelope::new(
                self.event.clone(),
                Box::new(TrackingAck {
                    ack_called: Arc::clone(&self.ack_called),
                }),
            );
            Box::pin(stream::iter(vec![Ok(envelope)]))
        }
    }

    let consumer = TrackingConsumer {
        event: movie_discovered(),
        ack_called: Arc::clone(&ack_called),
    };

    WorkerService::new(Arc::new(consumer), vec![]).run().await;

    assert!(*ack_called.lock().unwrap());
}
