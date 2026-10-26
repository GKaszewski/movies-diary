use activitypub::objects::{ReviewApInput, review_to_ap_object};
use chrono::NaiveDateTime;
use domain::models::{PersistedReview, Review, ReviewSource};
use domain::value_objects::{Comment, MovieId, Rating, ReviewId, UserId};
use serde_json::{Value, json};

const PUBLIC: &str = "https://www.w3.org/ns/activitystreams#Public";

fn timestamp() -> NaiveDateTime {
    NaiveDateTime::parse_from_str("2024-06-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
}

fn review(rating: u8, comment: Option<&str>, spoilers: bool) -> Review {
    Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
        user_id: UserId::from_uuid(uuid::Uuid::new_v4()),
        rating: Rating::new(rating).unwrap(),
        comment: comment.map(|c| Comment::new(c.into()).unwrap()),
        watched_at: timestamp(),
        created_at: timestamp(),
        source: ReviewSource::Local,
        watch_medium: None,
        spoilers,
    })
}

fn input(movie_title: &str, actor_url: &str) -> ReviewApInput {
    ReviewApInput {
        ap_id: "https://example.com/reviews/1".parse().unwrap(),
        actor_url: actor_url.parse().unwrap(),
        movie_title: movie_title.to_string(),
        release_year: 2021,
        external_metadata_id: None,
        poster_url: None,
        base_url: "https://example.com".to_string(),
    }
}

fn render(review: &Review, input: ReviewApInput) -> Value {
    serde_json::to_value(review_to_ap_object(review, input)).unwrap()
}

fn tag_names(json: &Value) -> Vec<&str> {
    json["tag"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect()
}

#[test]
fn review_object_tags_movie_title_with_non_alphanumerics_stripped() {
    for (title, hashtag) in [
        ("The Dark Knight", "#TheDarkKnight"),
        ("Schindler's List", "#SchindlersList"),
        ("2001: A Space Odyssey", "#2001ASpaceOdyssey"),
    ] {
        let json = render(
            &review(4, None, false),
            input(title, "https://example.com/users/1"),
        );
        assert!(
            tag_names(&json).contains(&hashtag),
            "{title}: {:?}",
            tag_names(&json)
        );
    }
}

#[test]
fn review_object_includes_two_hashtags() {
    let json = render(
        &review(4, None, false),
        input("Dune", "https://example.com/users/1"),
    );

    assert_eq!(tag_names(&json), vec!["#MoviesDiary", "#Dune"]);
}

#[test]
fn review_object_is_publicly_addressed_with_followers_in_cc() {
    let json = render(
        &review(3, None, false),
        input("Dune", "https://example.com/users/abc"),
    );

    assert_eq!(json["to"], json!([PUBLIC]));
    assert_eq!(
        json["cc"],
        json!(["https://example.com/users/abc/followers"])
    );
}

#[test]
fn review_object_carries_spoiler_flag_and_summary_only_with_a_comment() {
    let json = render(
        &review(4, Some("the mentor is the villain"), true),
        input("Dune", "https://example.com/users/1"),
    );
    assert_eq!(json["containsSpoilers"], Value::Bool(true));
    assert_eq!(json["summary"], "Contains spoilers");

    // A flag without a comment never travels: there is nothing to veil.
    let json = render(
        &review(4, None, true),
        input("Dune", "https://example.com/users/1"),
    );
    assert!(json.get("containsSpoilers").is_none());
    assert!(json.get("summary").is_none());
}
