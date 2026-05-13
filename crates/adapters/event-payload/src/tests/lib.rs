use super::*;

fn fixed_dt() -> NaiveDateTime {
    chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .unwrap()
        .naive_utc()
}

fn review_logged() -> DomainEvent {
    DomainEvent::ReviewLogged {
        review_id: ReviewId::from_uuid(Uuid::new_v4()),
        movie_id: MovieId::from_uuid(Uuid::new_v4()),
        user_id: UserId::from_uuid(Uuid::new_v4()),
        rating: Rating::new(4).unwrap(),
        watched_at: fixed_dt(),
    }
}

fn review_updated() -> DomainEvent {
    DomainEvent::ReviewUpdated {
        review_id: ReviewId::from_uuid(Uuid::new_v4()),
        movie_id: MovieId::from_uuid(Uuid::new_v4()),
        user_id: UserId::from_uuid(Uuid::new_v4()),
        rating: Rating::new(3).unwrap(),
        watched_at: fixed_dt(),
    }
}

fn movie_discovered() -> DomainEvent {
    DomainEvent::MovieDiscovered {
        movie_id: MovieId::from_uuid(Uuid::new_v4()),
        external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
    }
}

fn round_trip(event: DomainEvent) {
    let payload = EventPayload::from(&event);
    let json = serde_json::to_string(&payload).expect("serialize");
    let back: EventPayload = serde_json::from_str(&json).expect("deserialize");
    let recovered = DomainEvent::try_from(back).expect("try_from");
    assert_eq!(EventPayload::from(&event), EventPayload::from(&recovered));
}

#[test]
fn round_trip_review_logged() {
    round_trip(review_logged());
}

#[test]
fn round_trip_review_updated() {
    round_trip(review_updated());
}

#[test]
fn round_trip_movie_discovered() {
    round_trip(movie_discovered());
}

#[test]
fn serialized_format_is_tagged() {
    let payload = EventPayload::from(&movie_discovered());
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains(r#""type":"MovieDiscovered""#));
    assert!(json.contains(r#""data":"#));
}

#[test]
fn event_type_strings() {
    assert_eq!(
        EventPayload::from(&review_logged()).event_type(),
        "ReviewLogged"
    );
    assert_eq!(
        EventPayload::from(&review_updated()).event_type(),
        "ReviewUpdated"
    );
    assert_eq!(
        EventPayload::from(&movie_discovered()).event_type(),
        "MovieDiscovered"
    );
}

#[test]
fn round_trip_image_stored() {
    let event = DomainEvent::ImageStored {
        key: "avatars/abc123".into(),
    };
    let payload = EventPayload::from(&event);
    let json = serde_json::to_string(&payload).unwrap();
    let back: EventPayload = serde_json::from_str(&json).unwrap();
    let recovered = DomainEvent::try_from(back).unwrap();
    assert_eq!(EventPayload::from(&event), EventPayload::from(&recovered));
}

#[test]
fn image_stored_event_type() {
    let payload = EventPayload::from(&DomainEvent::ImageStored {
        key: "posters/x".into(),
    });
    assert_eq!(payload.event_type(), "ImageStored");
}
