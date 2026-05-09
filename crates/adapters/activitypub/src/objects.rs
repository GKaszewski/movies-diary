use activitypub_federation::kinds::object::NoteType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use domain::models::Review;

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
    }
}
