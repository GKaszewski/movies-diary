use super::*;

#[test]
fn normalize_hashtag_strips_non_alphanumeric() {
    assert_eq!(normalize_hashtag("The Dark Knight"), "TheDarkKnight");
    assert_eq!(normalize_hashtag("Schindler's List"), "SchindlersList");
    assert_eq!(
        normalize_hashtag("2001: A Space Odyssey"),
        "2001ASpaceOdyssey"
    );
}

#[test]
fn review_to_ap_object_includes_two_hashtags() {
    use chrono::NaiveDateTime;
    use domain::{
        models::{PersistedReview, Review, ReviewSource},
        value_objects::{MovieId, Rating, ReviewId, UserId},
    };

    let review = Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
        user_id: UserId::from_uuid(uuid::Uuid::new_v4()),
        rating: Rating::new(4).unwrap(),
        comment: None,
        watched_at: NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap(),
        created_at: NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap(),
        source: ReviewSource::Local,
        watch_medium: None,
        spoilers: false,
    });
    let obj = review_to_ap_object(
        &review,
        ReviewApInput {
            ap_id: "https://example.com/reviews/1".parse().unwrap(),
            actor_url: "https://example.com/users/1".parse().unwrap(),
            movie_title: "Dune".to_string(),
            release_year: 2021,
            external_metadata_id: None,
            poster_url: None,
            base_url: "https://example.com".to_string(),
        },
    );
    assert_eq!(obj.tag.len(), 2);
    let names: Vec<&str> = obj.tag.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"#MoviesDiary"));
    assert!(names.contains(&"#Dune"));
}

#[test]
fn review_to_ap_object_has_public_addressing() {
    use chrono::NaiveDateTime;
    use domain::{
        models::{PersistedReview, Review, ReviewSource},
        value_objects::{MovieId, Rating, ReviewId, UserId},
    };

    let review = Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
        user_id: UserId::from_uuid(uuid::Uuid::new_v4()),
        rating: Rating::new(3).unwrap(),
        comment: None,
        watched_at: NaiveDateTime::parse_from_str("2024-06-01 00:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap(),
        created_at: NaiveDateTime::parse_from_str("2024-06-01 00:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap(),
        source: ReviewSource::Local,
        watch_medium: None,
        spoilers: false,
    });
    let actor_url: url::Url = "https://example.com/users/abc".parse().unwrap();
    let obj = review_to_ap_object(
        &review,
        ReviewApInput {
            ap_id: "https://example.com/reviews/1".parse().unwrap(),
            actor_url: actor_url.clone(),
            movie_title: "Dune".to_string(),
            release_year: 2021,
            external_metadata_id: None,
            poster_url: None,
            base_url: "https://example.com".to_string(),
        },
    );
    assert_eq!(obj.to, vec!["https://www.w3.org/ns/activitystreams#Public"]);
    assert_eq!(obj.cc, vec!["https://example.com/users/abc/followers"]);
}

#[test]
fn review_to_ap_object_carries_spoiler_flag_and_summary() {
    use chrono::NaiveDateTime;
    use domain::{
        models::{PersistedReview, Review, ReviewSource},
        value_objects::{Comment, MovieId, Rating, ReviewId, UserId},
    };

    let ts = NaiveDateTime::parse_from_str("2024-06-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let review = Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
        user_id: UserId::from_uuid(uuid::Uuid::new_v4()),
        rating: Rating::new(4).unwrap(),
        comment: Some(Comment::new("the mentor is the villain".into()).unwrap()),
        watched_at: ts,
        created_at: ts,
        source: ReviewSource::Local,
        watch_medium: None,
        spoilers: true,
    });
    let obj = review_to_ap_object(
        &review,
        ReviewApInput {
            ap_id: "https://example.com/reviews/1".parse().unwrap(),
            actor_url: "https://example.com/users/1".parse().unwrap(),
            movie_title: "Dune".to_string(),
            release_year: 2021,
            external_metadata_id: None,
            poster_url: None,
            base_url: "https://example.com".to_string(),
        },
    );
    assert!(obj.contains_spoilers);
    assert_eq!(obj.summary.as_deref(), Some("Contains spoilers"));
    let json = serde_json::to_value(&obj).unwrap();
    assert_eq!(json["containsSpoilers"], serde_json::Value::Bool(true));
    assert_eq!(json["summary"], "Contains spoilers");

    // A flag without a comment never travels: there is nothing to veil.
    let review_no_comment = Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
        user_id: UserId::from_uuid(uuid::Uuid::new_v4()),
        rating: Rating::new(4).unwrap(),
        comment: None,
        watched_at: ts,
        created_at: ts,
        source: ReviewSource::Local,
        watch_medium: None,
        spoilers: true,
    });
    let obj2 = review_to_ap_object(
        &review_no_comment,
        ReviewApInput {
            ap_id: "https://example.com/reviews/2".parse().unwrap(),
            actor_url: "https://example.com/users/1".parse().unwrap(),
            movie_title: "Dune".to_string(),
            release_year: 2021,
            external_metadata_id: None,
            poster_url: None,
            base_url: "https://example.com".to_string(),
        },
    );
    assert!(!obj2.contains_spoilers);
    assert!(obj2.summary.is_none());
}

#[test]
fn watchlist_to_ap_object_has_public_addressing() {
    let actor_url: url::Url = "https://example.com/users/abc".parse().unwrap();
    let obj = watchlist_to_ap_object(WatchlistApInput {
        ap_id: "https://example.com/watchlist/1".parse().unwrap(),
        actor_url: actor_url.clone(),
        movie_title: "Alien".to_string(),
        release_year: 1979,
        external_metadata_id: None,
        poster_url: None,
        added_at: chrono::Utc::now(),
        base_url: "https://example.com".to_string(),
    });
    assert_eq!(obj.to, vec!["https://www.w3.org/ns/activitystreams#Public"]);
    assert_eq!(obj.cc, vec!["https://example.com/users/abc/followers"]);
}
