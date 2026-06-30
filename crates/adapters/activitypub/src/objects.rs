use chrono::{DateTime, Utc};
use k_ap::AS_PUBLIC;
use k_ap::NoteType;
use serde::{Deserialize, Serialize};
use url::Url;

use domain::models::Review;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApAttachment {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}

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
    pub(crate) external_metadata_id: Option<String>,
    #[serde(default)]
    pub(crate) poster_url: Option<String>,
    pub(crate) rating: u8,
    pub(crate) comment: Option<String>,
    pub(crate) watched_at: DateTime<Utc>,
    /// Discriminator so Movies Diary instances detect this as a review Note.
    #[serde(default)]
    pub(crate) review: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) attachment: Vec<ApAttachment>,
    #[serde(default)]
    pub(crate) tag: Vec<ApHashtag>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) cc: Vec<String>,
}

pub struct ReviewApInput {
    pub ap_id: Url,
    pub actor_url: Url,
    pub movie_title: String,
    pub release_year: u16,
    pub external_metadata_id: Option<String>,
    pub poster_url: Option<String>,
    pub base_url: String,
}

pub fn review_to_ap_object(review: &Review, input: ReviewApInput) -> ReviewObject {
    let ReviewApInput {
        ap_id,
        actor_url,
        movie_title,
        release_year,
        external_metadata_id,
        poster_url,
        base_url,
    } = input;
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
            href: Url::parse(&format!("{}/tags/moviesdiary", &base_url)).expect("valid base_url"),
            name: "#MoviesDiary".to_string(),
        },
        ApHashtag {
            kind: "Hashtag".to_string(),
            href: Url::parse(&format!("{}/tags/{}", &base_url, normalized.to_lowercase()))
                .expect("valid base_url"),
            name: format!("#{}", normalized),
        },
    ];
    let attachment = match &poster_url {
        Some(url) => vec![ApAttachment {
            kind: "Image".to_string(),
            url: url.clone(),
            media_type: Some("image/jpeg".to_string()),
            name: Some(movie_title.clone()),
        }],
        None => vec![],
    };

    ReviewObject {
        kind: NoteType::default(),
        id: ap_id,
        attributed_to: actor_url.clone(),
        content,
        published: DateTime::from_naive_utc_and_offset(*review.created_at(), Utc),
        movie_title,
        release_year,
        external_metadata_id,
        poster_url,
        rating: review.rating().value(),
        comment: comment_text,
        watched_at: DateTime::from_naive_utc_and_offset(*review.watched_at(), Utc),
        review: true,
        attachment,
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

pub struct WatchlistApInput {
    pub ap_id: Url,
    pub actor_url: Url,
    pub movie_title: String,
    pub release_year: u16,
    pub external_metadata_id: Option<String>,
    pub poster_url: Option<String>,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub base_url: String,
}

pub fn watchlist_to_ap_object(input: WatchlistApInput) -> WatchlistObject {
    let WatchlistApInput {
        ap_id,
        actor_url,
        movie_title,
        release_year,
        external_metadata_id,
        poster_url,
        added_at,
        base_url,
    } = input;
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

// ── Goal object ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalObject {
    #[serde(rename = "type")]
    pub(crate) kind: NoteType,
    pub(crate) id: Url,
    pub(crate) attributed_to: Url,
    pub(crate) content: String,
    pub(crate) published: chrono::DateTime<chrono::Utc>,
    pub(crate) goal_year: u16,
    pub(crate) goal_target: u32,
    pub(crate) goal_current: u32,
    #[serde(default)]
    pub(crate) goal: bool,
    #[serde(default)]
    pub(crate) tag: Vec<ApHashtag>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) cc: Vec<String>,
}

pub fn goal_to_ap_object(
    ap_id: Url,
    actor_url: Url,
    year: u16,
    target: u32,
    current: u32,
    base_url: &str,
) -> GoalObject {
    let content = format!(
        "🎯 Goal: Watch {} movies in {} ({}/{})",
        target, year, current, target
    );
    let tag = vec![ApHashtag {
        kind: "Hashtag".to_string(),
        href: Url::parse(&format!("{}/tags/moviesdiary", base_url)).expect("valid base_url"),
        name: "#MoviesDiary".to_string(),
    }];

    GoalObject {
        kind: NoteType::default(),
        id: ap_id,
        attributed_to: actor_url.clone(),
        content,
        published: chrono::Utc::now(),
        goal_year: year,
        goal_target: target,
        goal_current: current,
        goal: true,
        tag,
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![format!("{}/followers", actor_url)],
    }
}

#[cfg(test)]
#[path = "tests/objects.rs"]
mod tests;
