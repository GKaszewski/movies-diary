use super::*;
use crate::value_objects::{Email, PasswordHash, UserId, Username};

fn make_user() -> User {
    User::from_persistence(
        UserId::generate(),
        Email::new("a@b.com".to_string()).unwrap(),
        Username::new("alice".to_string()).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
        UserRole::Standard,
        UserProfile::default(),
    )
}

#[test]
fn update_profile_sets_fields() {
    let mut user = make_user();
    user.update_profile(UserProfile {
        bio: Some("My bio".to_string()),
        avatar_path: Some("avatars/abc".to_string()),
        ..Default::default()
    });
    assert_eq!(user.bio(), Some("My bio"));
    assert_eq!(user.avatar_path(), Some("avatars/abc"));
}

#[test]
fn update_profile_clears_with_none() {
    let mut user = make_user();
    user.update_profile(UserProfile {
        bio: Some("bio".to_string()),
        avatar_path: Some("path".to_string()),
        ..Default::default()
    });
    user.update_profile(UserProfile::default());
    assert_eq!(user.bio(), None);
    assert_eq!(user.avatar_path(), None);
}

// ── Movie ────────────────────────────────────────────────────────────────────

fn make_movie() -> Movie {
    Movie::new(
        Some(crate::value_objects::ExternalMetadataId::new("tt1234567".into()).unwrap()),
        crate::value_objects::MovieTitle::new("Blade Runner".into()).unwrap(),
        crate::value_objects::ReleaseYear::new(1982).unwrap(),
        Some("Ridley Scott".into()),
        Some(crate::value_objects::PosterPath::new("/poster.jpg".into()).unwrap()),
    )
}

#[test]
fn movie_new_sets_fields() {
    let m = make_movie();
    assert_eq!(m.title().value(), "Blade Runner");
    assert_eq!(m.release_year().value(), 1982);
    assert_eq!(m.director(), Some("Ridley Scott"));
    assert!(m.poster_path().is_some());
    assert!(m.external_metadata_id().is_some());
}

#[test]
fn movie_update_poster() {
    let mut m = make_movie();
    let new_poster = crate::value_objects::PosterPath::new("/new.jpg".into()).unwrap();
    m.update_poster(new_poster);
    assert_eq!(m.poster_path().unwrap().value(), "/new.jpg");
}

// ── Review ───────────────────────────────────────────────────────────────────

use crate::value_objects::{Comment, MovieId, Rating, ReviewId};

fn make_review() -> Review {
    Review::new(
        MovieId::generate(),
        UserId::generate(),
        Rating::new(4).unwrap(),
        Some(Comment::new("great".into()).unwrap()),
        chrono::Utc::now().naive_utc(),
    )
    .unwrap()
}

#[test]
fn review_new_sets_local_source() {
    let r = make_review();
    assert_eq!(*r.source(), ReviewSource::Local);
    assert!(!r.is_remote());
}

#[test]
fn review_stars() {
    let r = make_review(); // rating=4
    assert_eq!(r.stars(), [true, true, true, true, false]);
}

#[test]
fn review_from_persistence() {
    let id = ReviewId::generate();
    let mid = MovieId::generate();
    let uid = UserId::generate();
    let ts = chrono::Utc::now().naive_utc();
    let r = Review::from_persistence(PersistedReview {
        id: id.clone(),
        movie_id: mid.clone(),
        user_id: uid.clone(),
        rating: Rating::new(2).unwrap(),
        comment: None,
        watched_at: ts,
        created_at: ts,
        source: ReviewSource::Remote {
            actor_url: "https://example.com/actor".into(),
        },
    });
    assert_eq!(*r.id(), id);
    assert!(r.is_remote());
    assert_eq!(r.comment(), None);
}

// ── User ─────────────────────────────────────────────────────────────────────

#[test]
fn user_new() {
    let u = User::new(
        Email::new("x@y.com".into()).unwrap(),
        Username::new("bob".into()).unwrap(),
        PasswordHash::new("hashed".into()).unwrap(),
        UserRole::Admin,
    );
    assert_eq!(u.email().value(), "x@y.com");
    assert_eq!(u.username().value(), "bob");
    assert_eq!(u.role().as_str(), "admin");
    assert_eq!(u.bio(), None);
}

#[test]
fn user_update_password() {
    let mut u = make_user();
    u.update_password(PasswordHash::new("new_hash".into()).unwrap());
    assert_eq!(u.password_hash().value(), "new_hash");
}

// ── GoalType ─────────────────────────────────────────────────────────────────

#[test]
fn goal_type_as_str() {
    assert_eq!(GoalType::Movies.as_str(), "movies");
}

#[test]
fn goal_type_from_str() {
    assert_eq!("movies".parse::<GoalType>().unwrap(), GoalType::Movies);
    assert!("invalid".parse::<GoalType>().is_err());
}

// ── UserRole ─────────────────────────────────────────────────────────────────

#[test]
fn user_role_as_str() {
    assert_eq!(UserRole::Standard.as_str(), "standard");
    assert_eq!(UserRole::Admin.as_str(), "admin");
}

// ── ProfileField ─────────────────────────────────────────────────────────────

#[test]
fn profile_field_construction() {
    let f = ProfileField {
        name: "Website".into(),
        value: "https://example.com".into(),
    };
    assert_eq!(f.name, "Website");
    assert_eq!(f.value, "https://example.com");
}

// ── MovieStats ───────────────────────────────────────────────────────────────

#[test]
fn movie_stats_construction() {
    let s = MovieStats {
        total_count: 100,
        avg_rating: Some(3.5),
        federated_count: 10,
        rating_histogram: [5, 10, 30, 40, 15],
    };
    assert_eq!(s.total_count, 100);
    assert_eq!(s.rating_histogram[4], 15);
}

// ── FeedEntry ────────────────────────────────────────────────────────────────

#[test]
fn feed_entry_display_name_from_email() {
    let entry = DiaryEntry::new(make_movie(), make_review());
    let fe = FeedEntry::new(entry, "alice@example.com".into());
    assert_eq!(fe.user_display_name(), "alice");
    assert_eq!(fe.user_email(), "alice@example.com");
}

// ── MonthActivity ────────────────────────────────────────────────────────────

#[test]
fn month_activity_construction() {
    let ma = MonthActivity {
        year_month: "2024-06".into(),
        month_label: "June".into(),
        count: 5,
        entries: vec![],
    };
    assert_eq!(ma.year_month, "2024-06");
    assert_eq!(ma.count, 5);
    assert!(ma.entries.is_empty());
}

// ── Movie::is_manual_match ───────────────────────────────────────────────────

#[test]
fn movie_is_manual_match_same_title_year() {
    let m = make_movie();
    let title = crate::value_objects::MovieTitle::new("Blade Runner".into()).unwrap();
    let year = crate::value_objects::ReleaseYear::new(1982).unwrap();
    assert!(m.is_manual_match(&title, &year, Some("ridley scott")));
}

#[test]
fn movie_is_manual_match_different_director_fails() {
    let m = make_movie();
    let title = crate::value_objects::MovieTitle::new("Blade Runner".into()).unwrap();
    let year = crate::value_objects::ReleaseYear::new(1982).unwrap();
    assert!(!m.is_manual_match(&title, &year, Some("Denis Villeneuve")));
}

// ── UserProfile field validation ─────────────────────────────────────────────

#[test]
fn profile_fields_validates_max_count() {
    let fields: Vec<ProfileField> = (0..5)
        .map(|i| ProfileField {
            name: format!("f{i}"),
            value: format!("v{i}"),
        })
        .collect();
    assert!(UserProfile::validate_custom_fields(&fields).is_err());
}

#[test]
fn profile_fields_allows_four() {
    let fields: Vec<ProfileField> = (0..4)
        .map(|i| ProfileField {
            name: format!("f{i}"),
            value: format!("v{i}"),
        })
        .collect();
    assert!(UserProfile::validate_custom_fields(&fields).is_ok());
}

#[test]
fn profile_fields_allows_zero() {
    assert!(UserProfile::validate_custom_fields(&[]).is_ok());
}
