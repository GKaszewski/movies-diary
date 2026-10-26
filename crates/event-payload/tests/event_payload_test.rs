use chrono::NaiveDateTime;
use domain::{
    events::DomainEvent,
    value_objects::{ExternalMetadataId, MovieId, Rating, ReviewId, UserId},
};
use event_payload::EventPayload;
use uuid::Uuid;

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

#[test]
fn a_message_round_trips_its_event_id_and_decodes_without_one() {
    use event_payload::EventMessage;

    let with_id = EventMessage {
        event_id: Some(Uuid::new_v4().to_string()),
        payload: EventPayload::from(&review_logged()),
    };
    let json = serde_json::to_string(&with_id).unwrap();
    assert_eq!(
        serde_json::from_str::<EventMessage>(&json).unwrap(),
        with_id
    );

    let legacy = serde_json::to_string(&EventPayload::from(&review_logged())).unwrap();
    let decoded: EventMessage = serde_json::from_str(&legacy).unwrap();
    assert_eq!(decoded.event_id, None);

    let unit = EventMessage {
        event_id: Some("x".into()),
        payload: EventPayload::SearchReindexRequested,
    };
    let json = serde_json::to_string(&unit).unwrap();
    assert_eq!(serde_json::from_str::<EventMessage>(&json).unwrap(), unit);
}

fn every_event() -> Vec<DomainEvent> {
    use domain::{
        models::{ExternalPersonId, PersonId},
        value_objects::{FollowTarget, GoalId, PosterPath, SocialIdentity, WrapUpId},
    };
    let user = || UserId::from_uuid(Uuid::new_v4());
    let movie = || MovieId::from_uuid(Uuid::new_v4());
    let local = || SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4()));
    let remote = || SocialIdentity::Remote {
        actor_url: "https://remote.example/users/a".into(),
    };
    let date = |d| chrono::NaiveDate::from_ymd_opt(2026, 1, d).unwrap();
    vec![
        review_logged(),
        review_updated(),
        movie_discovered(),
        DomainEvent::MovieDeleted {
            movie_id: movie(),
            poster_path: Some(PosterPath::new("p/x.jpg".into()).unwrap()),
        },
        DomainEvent::UserUpdated { user_id: user() },
        DomainEvent::ReviewDeleted {
            review_id: ReviewId::from_uuid(Uuid::new_v4()),
            user_id: user(),
        },
        DomainEvent::MovieEnrichmentRequested {
            movie_id: movie(),
            external_metadata_id: ExternalMetadataId::new("tt7654321".into()).unwrap(),
        },
        DomainEvent::PersonEnrichmentRequested {
            person_id: PersonId::from_uuid(Uuid::new_v4()),
            external_person_id: ExternalPersonId::new("42"),
        },
        DomainEvent::ImageStored { key: "k".into() },
        DomainEvent::WatchlistEntryAdded {
            user_id: user(),
            movie_id: movie(),
            movie_title: "Heat".into(),
            release_year: 1995,
            external_metadata_id: Some("tt0113277".into()),
            added_at: fixed_dt(),
        },
        DomainEvent::WatchlistEntryRemoved {
            user_id: user(),
            movie_id: movie(),
        },
        DomainEvent::FollowRequested {
            follower: user(),
            target: FollowTarget::Handle("a@b.example".into()),
        },
        DomainEvent::FollowAccepted {
            owner: user(),
            requester: remote(),
        },
        DomainEvent::FollowRejected {
            owner: user(),
            requester: local(),
        },
        DomainEvent::Unfollowed {
            follower: user(),
            target: remote(),
        },
        DomainEvent::FollowerRemoved {
            owner: user(),
            follower: local(),
        },
        DomainEvent::ActorBlocked {
            blocker: user(),
            target: remote(),
        },
        DomainEvent::ActorUnblocked {
            blocker: user(),
            target: remote(),
        },
        DomainEvent::BackfillFollower {
            owner_user_id: user(),
            follower_inbox_url: "https://remote.example/inbox".into(),
        },
        DomainEvent::FederationDeliveryRequested {
            inbox_url: "https://remote.example/inbox".into(),
            activity_json: serde_json::json!({"type": "Create"}),
            signing_actor_id: Uuid::new_v4(),
        },
        DomainEvent::WatchEventIngested {
            user_id: user(),
            title: "Heat".into(),
            source: "jellyfin".into(),
        },
        DomainEvent::WrapUpRequested {
            wrapup_id: WrapUpId::from_uuid(Uuid::new_v4()),
            user_id: Some(user()),
            start_date: date(1),
            end_date: date(2),
        },
        DomainEvent::WrapUpCompleted {
            wrapup_id: WrapUpId::from_uuid(Uuid::new_v4()),
        },
        DomainEvent::SearchReindexRequested,
        DomainEvent::PosterSynced { movie_id: movie() },
        DomainEvent::GoalCreated {
            goal_id: GoalId::from_uuid(Uuid::new_v4()),
            user_id: user(),
            year: 2026,
            target_count: 50,
        },
        DomainEvent::GoalUpdated {
            goal_id: GoalId::from_uuid(Uuid::new_v4()),
            user_id: user(),
            year: 2026,
            target_count: 60,
        },
        DomainEvent::GoalDeleted {
            goal_id: GoalId::from_uuid(Uuid::new_v4()),
            user_id: user(),
            year: 2026,
        },
        DomainEvent::UserDeleted { user_id: user() },
        DomainEvent::UserAccountMoved {
            user_id: user(),
            new_actor_url: "https://new.example/users/a".into(),
        },
    ]
}

fn assert_covers_every_variant(event: &DomainEvent) {
    use DomainEvent::*;
    match event {
        ReviewLogged { .. }
        | ReviewUpdated { .. }
        | MovieDiscovered { .. }
        | MovieDeleted { .. }
        | UserUpdated { .. }
        | ReviewDeleted { .. }
        | MovieEnrichmentRequested { .. }
        | PersonEnrichmentRequested { .. }
        | ImageStored { .. }
        | WatchlistEntryAdded { .. }
        | WatchlistEntryRemoved { .. }
        | FollowRequested { .. }
        | FollowAccepted { .. }
        | FollowRejected { .. }
        | Unfollowed { .. }
        | FollowerRemoved { .. }
        | ActorBlocked { .. }
        | ActorUnblocked { .. }
        | BackfillFollower { .. }
        | FederationDeliveryRequested { .. }
        | WatchEventIngested { .. }
        | WrapUpRequested { .. }
        | WrapUpCompleted { .. }
        | SearchReindexRequested
        | PosterSynced { .. }
        | GoalCreated { .. }
        | GoalUpdated { .. }
        | GoalDeleted { .. }
        | UserDeleted { .. }
        | UserAccountMoved { .. } => {}
    }
}

#[test]
fn every_event_round_trips_through_an_event_message_with_and_without_an_id() {
    use event_payload::EventMessage;

    let events = every_event();
    events.iter().for_each(assert_covers_every_variant);
    let types: std::collections::HashSet<_> = events
        .iter()
        .map(|e| EventPayload::from(e).event_type())
        .collect();
    assert_eq!(types.len(), events.len(), "one sample per variant");

    for event in &events {
        for id in [Some(Uuid::new_v4().to_string()), None] {
            let message = EventMessage {
                event_id: id.clone(),
                payload: EventPayload::from(event),
            };
            let json = serde_json::to_string(&message).unwrap();
            let back: EventMessage = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("{} decode: {e}", message.payload.event_type()));
            assert_eq!(back.event_id, id);
            let recovered = DomainEvent::try_from(back.payload).unwrap();
            assert_eq!(EventPayload::from(event), EventPayload::from(&recovered));
        }
    }
}
