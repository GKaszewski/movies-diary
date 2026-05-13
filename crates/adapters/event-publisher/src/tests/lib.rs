use super::*;
use domain::{
    events::DomainEvent,
    value_objects::{ExternalMetadataId, MovieId},
};
use futures::StreamExt;

fn movie_discovered() -> DomainEvent {
    DomainEvent::MovieDiscovered {
        movie_id: MovieId::generate(),
        external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
    }
}

#[tokio::test]
async fn consumer_yields_published_events() {
    let config = EventPublisherConfig { channel_buffer: 8 };
    let (publisher, consumer) = create_event_channel(config);

    publisher.publish(&movie_discovered()).await.unwrap();
    drop(publisher);

    let mut stream = consumer.consume();
    let envelope = stream.next().await.unwrap().unwrap();
    assert!(matches!(
        envelope.event,
        DomainEvent::MovieDiscovered { .. }
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn consumer_yields_multiple_events_in_order() {
    let config = EventPublisherConfig { channel_buffer: 8 };
    let (publisher, consumer) = create_event_channel(config);

    publisher.publish(&movie_discovered()).await.unwrap();
    publisher.publish(&movie_discovered()).await.unwrap();
    drop(publisher);

    let mut stream = consumer.consume();
    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();
    assert!(matches!(first.event, DomainEvent::MovieDiscovered { .. }));
    assert!(matches!(second.event, DomainEvent::MovieDiscovered { .. }));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn stream_ends_when_publisher_dropped() {
    let config = EventPublisherConfig { channel_buffer: 8 };
    let (publisher, consumer) = create_event_channel(config);
    drop(publisher);

    let mut stream = consumer.consume();
    assert!(stream.next().await.is_none());
}
