use chrono::NaiveDateTime;
use serde::Deserialize;
use uuid::Uuid;

use application::diary::{
    commands::{LogReviewCommand, MovieInput},
    queries::GetDiaryQuery,
};
use domain::{errors::DomainError, models::SortDirection};

use api_types::{DiaryQueryParams, LogReviewRequest};

pub fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = Option::<String>::deserialize(de)?;
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => s.parse::<T>().map(Some).map_err(serde::de::Error::custom),
    }
}

#[derive(Deserialize)]
pub struct LogReviewForm {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub external_metadata_id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub manual_title: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub manual_release_year: Option<u16>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub manual_director: Option<String>,
    pub rating: u8,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub comment: Option<String>,
    pub watched_at: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub watch_medium: Option<String>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub username: String,
    pub password: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct ErrorQuery {
    pub error: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct FeedQueryParams {
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub sort_by: String,
    #[serde(default)]
    pub search: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Deserialize, Default)]
pub struct DeleteRedirectForm {
    #[serde(default)]
    pub redirect_after: Option<String>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct FollowForm {
    pub handle: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
    #[serde(default)]
    pub redirect_after: Option<String>,
}

#[derive(Deserialize)]
pub struct UnfollowForm {
    pub actor_url: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct FollowerActionForm {
    pub actor_url: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct BlockDomainForm {
    pub domain: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct RemoveDomainForm {
    pub domain: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct ActorUrlForm {
    pub actor_url: String,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(serde::Deserialize)]
pub struct WatchlistAddForm {
    pub movie_id: Option<uuid::Uuid>,
    pub query: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub year: Option<u16>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
    #[serde(default)]
    pub redirect_after: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct WatchlistQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub error: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ProfileQueryParams {
    pub view: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub error: Option<String>,
    #[serde(default)]
    pub sort_by: String,
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub embed: bool,
}

pub struct LogReviewData {
    pub external_metadata_id: Option<String>,
    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: NaiveDateTime,
    pub watch_medium: Option<domain::value_objects::WatchMedium>,
}

#[derive(Debug)]
pub struct ParseReviewError {
    pub field: &'static str,
    pub message: String,
}

impl TryFrom<LogReviewForm> for LogReviewData {
    type Error = ParseReviewError;

    fn try_from(form: LogReviewForm) -> Result<Self, Self::Error> {
        let watched_at =
            domain::value_objects::parse_watched_at(&form.watched_at).map_err(|_| {
                ParseReviewError {
                    field: "watched_at",
                    message: format!(
                        "invalid date '{}'; expected YYYY-MM-DD or YYYY-MM-DDTHH:MM[:SS]",
                        form.watched_at
                    ),
                }
            })?;
        let watch_medium = form
            .watch_medium
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| ParseReviewError {
                field: "watch_medium",
                message: "invalid watch medium".into(),
            })?;
        Ok(Self {
            external_metadata_id: form.external_metadata_id.filter(|s| !s.trim().is_empty()),
            manual_title: form.manual_title,
            manual_release_year: form.manual_release_year,
            manual_director: form.manual_director,
            rating: form.rating,
            comment: form.comment,
            watched_at,
            watch_medium,
        })
    }
}

impl TryFrom<LogReviewRequest> for LogReviewData {
    type Error = DomainError;

    fn try_from(req: LogReviewRequest) -> Result<Self, Self::Error> {
        let watched_at = domain::value_objects::parse_watched_at(&req.watched_at)?;
        let watch_medium = req.watch_medium.map(|s| s.parse()).transpose()?;
        Ok(Self {
            external_metadata_id: req.external_metadata_id.filter(|s| !s.trim().is_empty()),
            manual_title: req.manual_title,
            manual_release_year: req.manual_release_year,
            manual_director: req.manual_director,
            rating: req.rating,
            comment: req.comment,
            watched_at,
            watch_medium,
        })
    }
}

impl LogReviewData {
    pub fn into_command(self, user_id: Uuid) -> LogReviewCommand {
        LogReviewCommand {
            user_id,
            input: MovieInput {
                movie_id: None,
                external_metadata_id: self.external_metadata_id,
                manual_title: self.manual_title,
                manual_release_year: self.manual_release_year,
                manual_director: self.manual_director,
            },
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            watch_medium: self.watch_medium,
        }
    }
}

pub fn to_diary_query(p: DiaryQueryParams) -> GetDiaryQuery {
    GetDiaryQuery {
        limit: p.limit,
        offset: p.offset,
        sort_by: p.sort_by.as_deref().map(|s| match s {
            "date_asc" | "asc" => SortDirection::Ascending,
            "rating_desc" => SortDirection::ByRatingDesc,
            "rating_asc" => SortDirection::ByRatingAsc,
            _ => SortDirection::Descending,
        }),
        movie_id: p.movie_id,
        user_id: None,
    }
}

// ── Integrations forms ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateTokenForm {
    pub provider: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct RevokeTokenForm {
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize, Default)]
pub struct IntegrationsQuery {
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfirmWatchForm {
    pub rating: u8,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct DismissWatchForm {
    #[serde(rename = "_csrf", default)]
    pub csrf_token: String,
}

#[cfg(test)]
#[path = "tests/forms.rs"]
mod tests;
