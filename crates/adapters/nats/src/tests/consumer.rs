use super::*;
use domain::value_objects::UserId;
use event_payload::{EventMessage, EventPayload};
use uuid::Uuid;

fn payload() -> EventPayload {
    EventPayload::from(&DomainEvent::UserUpdated {
        user_id: UserId::from_uuid(Uuid::new_v4()),
    })
}

fn encode(event_id: Option<&str>) -> Vec<u8> {
    serde_json::to_vec(&EventMessage {
        event_id: event_id.map(str::to_owned),
        payload: payload(),
    })
    .unwrap()
}

#[test]
fn decode_message_reads_the_event_id() {
    let id = Uuid::new_v4();
    let (decoded, event) = decode_message(&encode(Some(&id.to_string()))).unwrap();
    assert_eq!(decoded.unwrap().value(), id);
    assert!(matches!(event, DomainEvent::UserUpdated { .. }));
}

#[test]
fn decode_message_accepts_a_legacy_payload_without_an_id() {
    let legacy = serde_json::to_vec(&payload()).unwrap();
    let (decoded, _) = decode_message(&legacy).unwrap();
    assert!(decoded.is_none());
    let (decoded, _) = decode_message(&encode(None)).unwrap();
    assert!(decoded.is_none());
}

#[test]
fn decode_message_rejects_a_malformed_id() {
    let err = decode_message(&encode(Some("not-a-uuid"))).unwrap_err();
    assert!(err.to_string().contains("event_id"), "{err}");
}

#[test]
fn decode_message_rejects_garbage() {
    assert!(decode_message(b"{not json").is_err());
}

#[test]
fn stream_with_the_default_window_is_updated_and_keeps_other_fields() {
    let existing = StreamConfig {
        name: "S".into(),
        max_messages: 42,
        duplicate_window: std::time::Duration::from_secs(120),
        ..Default::default()
    };
    let updated = stream_config_update(&existing).unwrap();
    assert_eq!(updated.duplicate_window, DEDUP_WINDOW);
    assert_eq!(updated.max_messages, 42);
    assert_eq!(updated.name, "S");
}

#[test]
fn stream_with_the_right_window_is_left_alone() {
    let existing = StreamConfig {
        duplicate_window: DEDUP_WINDOW,
        ..Default::default()
    };
    assert!(stream_config_update(&existing).is_none());
}

#[test]
fn consumer_max_deliver_drift_is_updated() {
    let existing = jetstream::consumer::Config {
        durable_name: Some("c".into()),
        filter_subject: "a.>".into(),
        max_deliver: -1,
        ack_wait: ACK_WAIT,
        max_ack_pending: 77,
        description: Some("operator".into()),
        ..Default::default()
    };
    let updated = consumer_config_update(&existing, 5).unwrap();
    assert_eq!(updated.max_deliver, 5);
    assert_eq!(updated.durable_name.as_deref(), Some("c"));
    assert_eq!(updated.filter_subject, "a.>");
    assert_eq!(updated.max_ack_pending, 77);
    assert_eq!(updated.description.as_deref(), Some("operator"));
    let same = jetstream::consumer::Config {
        max_deliver: 5,
        ..existing
    };
    assert!(consumer_config_update(&same, 5).is_none());
}

#[test]
fn nak_delay_follows_the_shared_backoff_by_delivery_count() {
    let secs = |d| nak_plan(d, 5).delay.as_secs();
    assert_eq!((secs(1), secs(2), secs(3), secs(4)), (10, 20, 40, 80));
    assert_eq!(nak_plan(50, 100).delay.as_secs(), 300);
}

#[test]
fn only_the_final_delivery_is_exhausted() {
    assert!(!nak_plan(4, 5).exhausted);
    assert!(nak_plan(5, 5).exhausted);
    assert!(nak_plan(6, 5).exhausted);
    assert!(!nak_plan(1_000, -1).exhausted, "-1 means unlimited");
}
