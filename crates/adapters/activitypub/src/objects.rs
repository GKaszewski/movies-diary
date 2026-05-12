use activitypub_federation::kinds::object::NoteType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use domain::models::Review;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApHashtag {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) href: Url,
    pub(crate) name: String,
}

pub(crate) fn normalize_hashtag(title: &str) -> String {
    title.chars().filter(|c| c.is_alphanumeric()).collect()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewObject {
    #[serde(rename = "type")]
    pub(crate) kind: NoteType,
    pub(crate) id: Url,
    pub(crate) attributed_to: Url,
    pub(crate) content: String,
    pub(crate) published: DateTime<Utc>,
    pub(crate) movie_title: String,
    #[serde(default)]
    pub(crate) release_year: u16,
    #[serde(default)]
    pub(crate) poster_url: Option<String>,
    pub(crate) rating: u8,
    pub(crate) comment: Option<String>,
    pub(crate) watched_at: DateTime<Utc>,
    #[serde(default)]
    pub(crate) tag: Vec<ApHashtag>,
}

/// Serialize a local Review into a ReviewObject for AP delivery.
/// Takes movie metadata explicitly since the handler fetches it separately.
pub fn review_to_ap_object(
    review: &Review,
    ap_id: Url,
    actor_url: Url,
    movie_title: String,
    release_year: u16,
    poster_url: Option<String>,
    base_url: &str,
) -> ReviewObject {
    let stars: String = "\u{2B50}".repeat(review.rating().value() as usize);
    let comment_text = review.comment().map(|c| c.value().to_string());
    let year_str = if release_year > 0 {
        format!(" ({})", release_year)
    } else {
        String::new()
    };
    let watched_str = format!("Watched: {}", review.watched_at().format("%b %-d, %Y"));
    let content = match &comment_text {
        Some(c) => format!(
            "{} {}{}\n{}\n{}",
            stars, movie_title, year_str, c, watched_str
        ),
        None => format!("{} {}{}\n{}", stars, movie_title, year_str, watched_str),
    };

    let normalized = normalize_hashtag(&movie_title);
    let tag = vec![
        ApHashtag {
            kind: "Hashtag".to_string(),
            href: Url::parse(&format!("{}/tags/moviesdiary", base_url))
                .expect("valid base_url"),
            name: "#MoviesDiary".to_string(),
        },
        ApHashtag {
            kind: "Hashtag".to_string(),
            href: Url::parse(&format!("{}/tags/{}", base_url, normalized.to_lowercase()))
                .expect("valid base_url"),
            name: format!("#{}", normalized),
        },
    ];

    ReviewObject {
        kind: NoteType::default(),
        id: ap_id,
        attributed_to: actor_url,
        content,
        published: DateTime::from_naive_utc_and_offset(*review.created_at(), Utc),
        movie_title,
        release_year,
        poster_url,
        rating: review.rating().value(),
        comment: comment_text,
        watched_at: DateTime::from_naive_utc_and_offset(*review.watched_at(), Utc),
        tag,
    }
}

#[cfg(test)]
mod tests {
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
}
