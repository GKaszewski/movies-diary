use super::*;

fn ts() -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2024, 6, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

#[test]
fn watch_event_new_has_pending_status() {
    let e = WatchEvent::new(
        UserId::generate(),
        "Dune".into(),
        Some(2021),
        None,
        WatchEventSource::Jellyfin,
        ts(),
        None,
    );
    assert_eq!(*e.status(), WatchEventStatus::Pending);
}

#[test]
fn watch_event_getters() {
    let uid = UserId::generate();
    let mid = MovieId::generate();
    let e = WatchEvent::new(
        uid.clone(),
        "Arrival".into(),
        Some(2016),
        Some("ext123".into()),
        WatchEventSource::Plex,
        ts(),
        Some(mid.clone()),
    );
    assert_eq!(*e.user_id(), uid);
    assert_eq!(e.title(), "Arrival");
    assert_eq!(e.year(), Some(2016));
    assert_eq!(e.external_metadata_id(), Some("ext123"));
    assert_eq!(*e.source(), WatchEventSource::Plex);
    assert_eq!(e.watched_at(), &ts());
    assert_eq!(*e.movie_id().unwrap(), mid);
}

#[test]
fn webhook_token_new() {
    let uid = UserId::generate();
    let t = WebhookToken::new(
        uid.clone(),
        "hash123".into(),
        WatchEventSource::Jellyfin,
        Some("my server".into()),
    );
    assert_eq!(*t.user_id(), uid);
    assert_eq!(t.token_hash(), "hash123");
    assert_eq!(*t.provider(), WatchEventSource::Jellyfin);
    assert_eq!(t.label(), Some("my server"));
    assert!(t.last_used_at().is_none());
}

#[test]
fn webhook_token_from_persistence() {
    let id = WebhookTokenId::generate();
    let uid = UserId::generate();
    let created = ts();
    let used = chrono::NaiveDate::from_ymd_opt(2024, 7, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let t = WebhookToken::from_persistence(
        id.clone(),
        uid.clone(),
        "h".into(),
        WatchEventSource::Plex,
        None,
        created,
        Some(used),
    );
    assert_eq!(*t.id(), id);
    assert_eq!(*t.user_id(), uid);
    assert_eq!(t.token_hash(), "h");
    assert_eq!(*t.provider(), WatchEventSource::Plex);
    assert_eq!(t.label(), None);
    assert_eq!(t.created_at(), &created);
    assert_eq!(t.last_used_at(), Some(&used));
}

#[test]
fn watch_event_source_display() {
    assert_eq!(WatchEventSource::Jellyfin.to_string(), "jellyfin");
    assert_eq!(WatchEventSource::Plex.to_string(), "plex");
}

#[test]
fn watch_event_source_from_str() {
    assert_eq!(
        "jellyfin".parse::<WatchEventSource>().unwrap(),
        WatchEventSource::Jellyfin
    );
    assert_eq!(
        "plex".parse::<WatchEventSource>().unwrap(),
        WatchEventSource::Plex
    );
    assert!("unknown".parse::<WatchEventSource>().is_err());
}

#[test]
fn watch_event_status_display() {
    assert_eq!(WatchEventStatus::Pending.to_string(), "pending");
    assert_eq!(WatchEventStatus::Confirmed.to_string(), "confirmed");
    assert_eq!(WatchEventStatus::Dismissed.to_string(), "dismissed");
}

#[test]
fn watch_event_status_from_str() {
    for s in ["pending", "confirmed", "dismissed"] {
        let parsed: WatchEventStatus = s.parse().unwrap();
        assert_eq!(parsed.to_string(), s);
    }
    assert!("bogus".parse::<WatchEventStatus>().is_err());
}

#[test]
fn parsed_playback_event_fields() {
    let p = ParsedPlaybackEvent {
        title: "Matrix".into(),
        year: Some(1999),
        tmdb_id: Some("603".into()),
        imdb_id: Some("tt0133093".into()),
    };
    assert_eq!(p.title, "Matrix");
    assert_eq!(p.year, Some(1999));
    assert_eq!(p.tmdb_id.as_deref(), Some("603"));
    assert_eq!(p.imdb_id.as_deref(), Some("tt0133093"));
}
