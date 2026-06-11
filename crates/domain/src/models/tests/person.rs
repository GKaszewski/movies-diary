use super::*;

#[test]
fn person_new() {
    let ext = ExternalPersonId::new("tmdb:12345");
    let pid = PersonId::from_external(&ext);
    let p = Person::basic(
        pid,
        ext,
        "Keanu Reeves".into(),
        Some("Acting".into()),
        Some("/profiles/keanu.jpg".into()),
    );
    assert_eq!(p.name(), "Keanu Reeves");
    assert_eq!(p.known_for_department(), Some("Acting"));
    assert_eq!(p.profile_path(), Some("/profiles/keanu.jpg"));
    assert_eq!(p.external_id().value(), "tmdb:12345");
    assert_eq!(p.external_id().tmdb_id(), Some(12345));
}

#[test]
fn person_id_from_external() {
    let ext = ExternalPersonId::new("tmdb:99999");
    let pid = PersonId::from_external(&ext);
    // UUIDv5 is deterministic — just ensure it's a valid uuid
    let _ = pid.value();
}

#[test]
fn person_id_deterministic() {
    let ext = ExternalPersonId::new("tmdb:42");
    let a = PersonId::from_external(&ext);
    let b = PersonId::from_external(&ext);
    assert_eq!(a, b);
}

#[test]
fn person_credits_default_empty() {
    let ext = ExternalPersonId::new("tmdb:1");
    let pid = PersonId::from_external(&ext);
    let p = Person::basic(pid, ext, "Test".into(), None, None);
    let credits = PersonCredits {
        person: p,
        cast: vec![],
        crew: vec![],
    };
    assert!(credits.cast.is_empty());
    assert!(credits.crew.is_empty());
}

#[test]
fn external_person_id_tmdb_id_none_for_other() {
    let ext = ExternalPersonId::new("imdb:nm0000206");
    assert_eq!(ext.tmdb_id(), None);
}
