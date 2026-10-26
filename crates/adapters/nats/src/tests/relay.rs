use super::*;
use domain::value_objects::UserId;
use uuid::Uuid;

fn event() -> DomainEvent {
    DomainEvent::UserUpdated {
        user_id: UserId::from_uuid(Uuid::new_v4()),
    }
}

#[test]
fn relaying_the_same_row_twice_sets_the_same_msg_id() {
    let id = EventId::new();
    let event = event();
    let first = relay_message("p", Some(&id), &event).unwrap();
    let second = relay_message("p", Some(&id), &event).unwrap();
    assert_eq!(first.msg_id, Some(id.value().to_string()));
    assert_eq!(first.msg_id, second.msg_id);
    assert_eq!(first.subject, "p.user.updated");
}

#[test]
fn a_legacy_row_without_an_id_has_no_msg_id() {
    let message = relay_message("p", None, &event()).unwrap();
    assert_eq!(message.msg_id, None);
    let decoded: serde_json::Value = serde_json::from_slice(&message.payload).unwrap();
    assert!(decoded.get("event_id").is_none());
}

#[test]
fn the_body_carries_the_event_id_for_the_consumer() {
    let id = EventId::new();
    let message = relay_message("p", Some(&id), &event()).unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(&message.payload).unwrap();
    assert_eq!(decoded["event_id"], id.value().to_string());
    assert_eq!(decoded["type"], "UserUpdated");
}
