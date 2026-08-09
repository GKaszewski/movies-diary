use super::*;

#[test]
fn movie_id_generate_unique() {
    let a = MovieId::generate();
    let b = MovieId::generate();
    assert_ne!(a, b);
}

#[test]
fn rating_valid_range() {
    assert!(Rating::new(1).is_ok());
    assert!(Rating::new(5).is_ok());
    assert_eq!(Rating::new(3).unwrap().value(), 3);
}

#[test]
fn rating_invalid() {
    assert!(Rating::new(0).is_err());
    assert!(Rating::new(6).is_err());
    assert!(Rating::new(255).is_err());
}

#[test]
fn movie_title_valid() {
    let t = MovieTitle::new("Test".into());
    assert!(t.is_ok());
    assert_eq!(t.unwrap().value(), "Test");
}

#[test]
fn movie_title_empty_rejected() {
    assert!(MovieTitle::new("".into()).is_err());
    assert!(MovieTitle::new("   ".into()).is_err());
}

#[test]
fn release_year_valid() {
    assert!(ReleaseYear::new(2024).is_ok());
    assert_eq!(ReleaseYear::new(1888).unwrap().value(), 1888);
}

#[test]
fn release_year_too_early() {
    assert!(ReleaseYear::new(1887).is_err());
}

#[test]
fn email_valid() {
    let e = Email::new("a@b.com".into());
    assert!(e.is_ok());
    assert_eq!(e.unwrap().value(), "a@b.com");
}

#[test]
fn email_invalid() {
    assert!(Email::new("invalid".into()).is_err());
    assert!(Email::new("".into()).is_err());
}

#[test]
fn username_valid() {
    let u = Username::new("test".into());
    assert!(u.is_ok());
    assert_eq!(u.unwrap().value(), "test");
}

#[test]
fn username_lowercases() {
    assert_eq!(Username::new("Alice".into()).unwrap().value(), "alice");
}

#[test]
fn username_rejects_too_short() {
    assert!(Username::new("a".into()).is_err());
}

#[test]
fn username_rejects_special_chars() {
    assert!(Username::new("no spaces".into()).is_err());
    assert!(Username::new("no@at".into()).is_err());
}

#[test]
fn poster_path_valid() {
    let p = PosterPath::new("path/to/poster".into());
    assert!(p.is_ok());
    assert_eq!(p.unwrap().value(), "path/to/poster");
}

#[test]
fn poster_path_empty_rejected() {
    assert!(PosterPath::new("".into()).is_err());
}

#[test]
fn comment_valid() {
    let c = Comment::new("nice movie".into());
    assert!(c.is_ok());
    assert_eq!(c.unwrap().value(), "nice movie");
}

#[test]
fn comment_empty_is_ok() {
    // empty comment allowed — only max-length checked
    assert!(Comment::new("".into()).is_ok());
}

#[test]
fn external_metadata_id_valid() {
    let e = ExternalMetadataId::new("tt1234567".into());
    assert!(e.is_ok());
    assert_eq!(e.unwrap().value(), "tt1234567");
}

#[test]
fn external_metadata_id_empty_rejected() {
    assert!(ExternalMetadataId::new("".into()).is_err());
    assert!(ExternalMetadataId::new("   ".into()).is_err());
}

#[test]
fn password_hash_valid() {
    assert!(PasswordHash::new("hash".into()).is_ok());
}

#[test]
fn password_hash_empty_rejected() {
    assert!(PasswordHash::new("".into()).is_err());
}

#[test]
fn poster_url_valid() {
    let u = PosterUrl::new("https://img.com/poster.jpg".into());
    assert!(u.is_ok());
    assert_eq!(u.unwrap().value(), "https://img.com/poster.jpg");
}

#[test]
fn poster_url_empty_rejected() {
    assert!(PosterUrl::new("".into()).is_err());
}

#[test]
fn password_min_length_enforced() {
    assert!(Password::new("short".to_string()).is_err());
    assert!(Password::new("1234567".to_string()).is_err()); // 7 chars
}

#[test]
fn password_valid_at_eight_chars() {
    let p = Password::new("12345678".to_string());
    assert!(p.is_ok());
    assert_eq!(p.unwrap().value(), "12345678");
}

#[test]
fn password_value_preserves_content() {
    let raw = "supersecret!".to_string();
    assert_eq!(Password::new(raw.clone()).unwrap().value(), raw);
}

#[test]
fn watch_medium_parses_valid_strings() {
    let cases = [
        ("cinema", WatchMedium::Cinema),
        ("streaming", WatchMedium::Streaming),
        ("tv", WatchMedium::TV),
        ("physical_media", WatchMedium::PhysicalMedia),
        ("download", WatchMedium::Download),
        ("media_server", WatchMedium::MediaServer),
        ("other", WatchMedium::Other),
    ];
    for (input, expected) in cases {
        let parsed: WatchMedium = input.parse().unwrap();
        assert_eq!(parsed, expected);
    }
}

#[test]
fn watch_medium_rejects_invalid() {
    assert!("nonsense".parse::<WatchMedium>().is_err());
    assert!("".parse::<WatchMedium>().is_err());
}

#[test]
fn watch_medium_display_round_trips() {
    let variants = [
        WatchMedium::Cinema,
        WatchMedium::Streaming,
        WatchMedium::TV,
        WatchMedium::PhysicalMedia,
        WatchMedium::Download,
        WatchMedium::MediaServer,
        WatchMedium::Other,
    ];
    for v in variants {
        let s = v.to_string();
        let parsed: WatchMedium = s.parse().unwrap();
        assert_eq!(parsed, v);
    }
}

// ── InstanceIdentity ────────────────────────────────────────────────────────

fn instance() -> InstanceIdentity {
    InstanceIdentity::new("https://md.example")
}

#[test]
fn instance_normalizes_trailing_slash() {
    assert_eq!(
        InstanceIdentity::new("https://md.example/").base_url(),
        "https://md.example"
    );
    assert_eq!(
        InstanceIdentity::new("https://md.example").base_url(),
        "https://md.example"
    );
}

#[test]
fn instance_host_strips_scheme_and_path() {
    assert_eq!(instance().host(), "md.example");
    assert_eq!(
        InstanceIdentity::new("http://localhost:3000").host(),
        "localhost:3000"
    );
}

#[test]
fn instance_builds_actor_url_for_local_user() {
    let uid = UserId::from_uuid(uuid::Uuid::nil());
    assert_eq!(
        instance().actor_url_for(&uid),
        format!("https://md.example/users/{}", uuid::Uuid::nil())
    );
}

#[test]
fn instance_builds_handle_and_image_url() {
    assert_eq!(instance().handle_for("gabriel"), "@gabriel@md.example");
    assert_eq!(
        instance().image_url_for("avatars/a.webp"),
        "https://md.example/images/avatars/a.webp"
    );
}

#[test]
fn instance_identifies_own_actor_url_as_local() {
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());
    let url = instance().actor_url_for(&uid);
    assert_eq!(instance().identify(&url), SocialIdentity::Local(uid));
}

#[test]
fn instance_identifies_foreign_actor_url_as_remote() {
    let url = "https://other.example/users/bob";
    assert_eq!(
        instance().identify(url),
        SocialIdentity::Remote {
            actor_url: url.to_string()
        }
    );
}

#[test]
fn instance_identifies_own_host_with_bad_uuid_as_remote() {
    let url = "https://md.example/users/not-a-uuid";
    assert_eq!(
        instance().identify(url),
        SocialIdentity::Remote {
            actor_url: url.to_string()
        }
    );
}

#[test]
fn instance_actor_url_round_trips_both_variants() {
    let i = instance();
    let local = SocialIdentity::Local(UserId::from_uuid(uuid::Uuid::new_v4()));
    let remote = SocialIdentity::Remote {
        actor_url: "https://other.example/users/bob".into(),
    };
    assert_eq!(i.identify(&i.actor_url_of(&local)), local);
    assert_eq!(i.identify(&i.actor_url_of(&remote)), remote);
}

#[test]
fn instance_rejects_prefix_collision() {
    // A different instance whose base_url merely starts the same must not read as local.
    let i = InstanceIdentity::new("https://md.example");
    let url = "https://md.example.evil.test/users/00000000-0000-0000-0000-000000000000";
    assert!(matches!(i.identify(url), SocialIdentity::Remote { .. }));
}
