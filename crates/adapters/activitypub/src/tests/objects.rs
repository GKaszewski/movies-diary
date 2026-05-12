use super::*;

#[test]
fn normalize_hashtag_strips_non_alphanumeric() {
    assert_eq!(normalize_hashtag("The Dark Knight"), "TheDarkKnight");
    assert_eq!(normalize_hashtag("Schindler's List"), "SchindlersList");
    assert_eq!(normalize_hashtag("2001: A Space Odyssey"), "2001ASpaceOdyssey");
}

#[test]
fn review_to_ap_object_includes_two_hashtags() {
    use chrono::NaiveDateTime;
    use domain::{
        models::{Review, ReviewSource},
        value_objects::{MovieId, Rating, ReviewId, UserId},
    };

    let review = Review::from_persistence(
        ReviewId::generate(),
        MovieId::from_uuid(uuid::Uuid::new_v4()),
        UserId::from_uuid(uuid::Uuid::new_v4()),
        Rating::new(4).unwrap(),
        None,
        NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        ReviewSource::Local,
    );
    let obj = review_to_ap_object(
        &review,
        "https://example.com/reviews/1".parse().unwrap(),
        "https://example.com/users/1".parse().unwrap(),
        "Dune".to_string(),
        2021,
        None,
        "https://example.com",
    );
    assert_eq!(obj.tag.len(), 2);
    let names: Vec<&str> = obj.tag.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"#MoviesDiary"));
    assert!(names.contains(&"#Dune"));
}
