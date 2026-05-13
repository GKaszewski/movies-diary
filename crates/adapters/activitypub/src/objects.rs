use activitypub_base::AS_PUBLIC;
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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) cc: Vec<String>,
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
            href: Url::parse(&format!("{}/tags/moviesdiary", base_url)).expect("valid base_url"),
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
        attributed_to: actor_url.clone(),
        content,
        published: DateTime::from_naive_utc_and_offset(*review.created_at(), Utc),
        movie_title,
        release_year,
        poster_url,
        rating: review.rating().value(),
        comment: comment_text,
        watched_at: DateTime::from_naive_utc_and_offset(*review.watched_at(), Utc),
        tag,
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![format!("{}/followers", actor_url)],
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistObject {
    #[serde(rename = "type")]
    pub(crate) kind: NoteType,
    pub(crate) id: Url,
    pub(crate) attributed_to: Url,
    pub(crate) content: String,
    pub(crate) published: chrono::DateTime<chrono::Utc>,
    pub(crate) movie_title: String,
    #[serde(default)]
    pub(crate) release_year: u16,
    #[serde(default)]
    pub(crate) external_metadata_id: Option<String>,
    #[serde(default)]
    pub(crate) poster_url: Option<String>,
    #[serde(default)]
    pub(crate) tag: Vec<ApHashtag>,
    /// Discriminator so Movies Diary instances distinguish this from a review Note.
    /// Non-Movies-Diary apps ignore unknown fields.
    #[serde(default)]
    pub(crate) watchlist_entry: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) cc: Vec<String>,
}

pub fn watchlist_to_ap_object(
    ap_id: Url,
    actor_url: Url,
    movie_title: String,
    release_year: u16,
    external_metadata_id: Option<String>,
    poster_url: Option<String>,
    added_at: chrono::DateTime<chrono::Utc>,
    base_url: &str,
) -> WatchlistObject {
    let year_str = if release_year > 0 {
        format!(" ({})", release_year)
    } else {
        String::new()
    };
    let content = format!("📋 {}{} — want to watch", movie_title, year_str);
    let normalized = normalize_hashtag(&movie_title);
    let tag = vec![
        ApHashtag {
            kind: "Hashtag".to_string(),
            href: Url::parse(&format!("{}/tags/moviesdiary", base_url)).expect("valid base_url"),
            name: "#MoviesDiary".to_string(),
        },
        ApHashtag {
            kind: "Hashtag".to_string(),
            href: Url::parse(&format!("{}/tags/{}", base_url, normalized.to_lowercase()))
                .expect("valid base_url"),
            name: format!("#{}", normalized),
        },
    ];

    WatchlistObject {
        kind: NoteType::default(),
        id: ap_id,
        attributed_to: actor_url.clone(),
        content,
        published: added_at,
        movie_title,
        release_year,
        external_metadata_id,
        poster_url,
        tag,
        watchlist_entry: true,
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![format!("{}/followers", actor_url)],
    }
}

#[cfg(test)]
#[path = "tests/objects.rs"]
mod tests;
