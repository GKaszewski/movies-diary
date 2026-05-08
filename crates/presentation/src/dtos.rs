use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use application::{commands::LogReviewCommand, queries::GetDiaryQuery};
use domain::{errors::DomainError, models::SortDirection};

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
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
pub struct DiaryQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub movie_id: Option<Uuid>,
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
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ErrorQuery {
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct LogReviewRequest {
    pub external_metadata_id: Option<String>,
    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
}

#[derive(Serialize)]
pub struct MovieDto {
    pub id: Uuid,
    pub title: String,
    pub release_year: u16,
    pub director: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Serialize)]
pub struct ReviewDto {
    pub id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
}

#[derive(Serialize)]
pub struct DiaryEntryDto {
    pub movie: MovieDto,
    pub review: ReviewDto,
}

#[derive(Serialize)]
pub struct DiaryResponse {
    pub items: Vec<DiaryEntryDto>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Serialize)]
pub struct ReviewHistoryResponse {
    pub movie: MovieDto,
    pub viewings: Vec<ReviewDto>,
    pub trend: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

pub struct LogReviewData {
    pub external_metadata_id: Option<String>,
    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct ParseReviewError {
    pub field: &'static str,
    pub message: String,
}

impl TryFrom<LogReviewForm> for LogReviewData {
    type Error = ParseReviewError;

    fn try_from(form: LogReviewForm) -> Result<Self, Self::Error> {
        let watched_at = NaiveDateTime::parse_from_str(&form.watched_at, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(&form.watched_at, "%Y-%m-%dT%H:%M"))
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(&form.watched_at, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight always valid"))
            })
            .map_err(|_| ParseReviewError {
                field: "watched_at",
                message: format!(
                    "invalid date '{}'; expected YYYY-MM-DD or YYYY-MM-DDTHH:MM[:SS]",
                    form.watched_at
                ),
            })?;
        Ok(Self {
            external_metadata_id: form.external_metadata_id.filter(|s| !s.trim().is_empty()),
            manual_title: form.manual_title,
            manual_release_year: form.manual_release_year,
            manual_director: form.manual_director,
            rating: form.rating,
            comment: form.comment,
            watched_at,
        })
    }
}

impl TryFrom<LogReviewRequest> for LogReviewData {
    type Error = DomainError;

    fn try_from(req: LogReviewRequest) -> Result<Self, Self::Error> {
        let watched_at = NaiveDateTime::parse_from_str(&req.watched_at, "%Y-%m-%dT%H:%M:%S")
            .map_err(|_| {
                DomainError::ValidationError(
                    "invalid watched_at; expected YYYY-MM-DDTHH:MM:SS".into(),
                )
            })?;
        Ok(Self {
            external_metadata_id: req.external_metadata_id.filter(|s| !s.trim().is_empty()),
            manual_title: req.manual_title,
            manual_release_year: req.manual_release_year,
            manual_director: req.manual_director,
            rating: req.rating,
            comment: req.comment,
            watched_at,
        })
    }
}

impl LogReviewData {
    pub fn into_command(self, user_id: Uuid) -> LogReviewCommand {
        LogReviewCommand {
            external_metadata_id: self.external_metadata_id,
            manual_title: self.manual_title,
            manual_release_year: self.manual_release_year,
            manual_director: self.manual_director,
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            user_id,
        }
    }
}

impl From<DiaryQueryParams> for GetDiaryQuery {
    fn from(p: DiaryQueryParams) -> Self {
        GetDiaryQuery {
            limit: p.limit,
            offset: p.offset,
            sort_by: p.sort_by.as_deref().map(|s| {
                if s == "asc" {
                    SortDirection::Ascending
                } else {
                    SortDirection::Descending
                }
            }),
            movie_id: p.movie_id,
            user_id: None,
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct ProfileQueryParams {
    pub view: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_form(watched_at: &str) -> LogReviewForm {
        LogReviewForm {
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
            rating: 4,
            comment: None,
            watched_at: watched_at.to_string(),
        }
    }

    fn make_request(watched_at: &str) -> LogReviewRequest {
        LogReviewRequest {
            external_metadata_id: None,
            manual_title: None,
            manual_release_year: None,
            manual_director: None,
            rating: 4,
            comment: None,
            watched_at: watched_at.to_string(),
        }
    }

    #[test]
    fn form_accepts_datetime_with_seconds() {
        let data = LogReviewData::try_from(make_form("2024-03-15T20:30:00")).unwrap();
        assert_eq!(data.watched_at.format("%H:%M:%S").to_string(), "20:30:00");
    }

    #[test]
    fn form_accepts_datetime_without_seconds() {
        let data = LogReviewData::try_from(make_form("2024-03-15T20:30")).unwrap();
        assert_eq!(data.watched_at.format("%H:%M").to_string(), "20:30");
    }

    #[test]
    fn form_rejects_invalid_datetime() {
        assert!(LogReviewData::try_from(make_form("not-a-date")).is_err());
    }

    #[test]
    fn api_accepts_datetime_with_seconds() {
        let data = LogReviewData::try_from(make_request("2024-03-15T20:30:00")).unwrap();
        assert_eq!(data.watched_at.format("%H:%M:%S").to_string(), "20:30:00");
    }

    #[test]
    fn api_rejects_datetime_without_seconds() {
        assert!(LogReviewData::try_from(make_request("2024-03-15T20:30")).is_err());
    }

    #[test]
    fn api_rejects_invalid_datetime() {
        assert!(LogReviewData::try_from(make_request("garbage")).is_err());
    }

    #[test]
    fn whitespace_external_id_becomes_none_in_form() {
        let mut form = make_form("2024-03-15T20:30:00");
        form.external_metadata_id = Some("   ".to_string());
        let data = LogReviewData::try_from(form).unwrap();
        assert!(data.external_metadata_id.is_none());
    }

    #[test]
    fn whitespace_external_id_becomes_none_in_request() {
        let mut req = make_request("2024-03-15T20:30:00");
        req.external_metadata_id = Some("   ".to_string());
        let data = LogReviewData::try_from(req).unwrap();
        assert!(data.external_metadata_id.is_none());
    }

    #[test]
    fn into_command_sets_user_id() {
        let data = LogReviewData::try_from(make_form("2024-03-15T20:30:00")).unwrap();
        let user_id = Uuid::new_v4();
        let cmd = data.into_command(user_id);
        assert_eq!(cmd.user_id, user_id);
    }

    #[test]
    fn sort_by_asc_string_becomes_ascending() {
        let params = DiaryQueryParams {
            sort_by: Some("asc".to_string()),
            limit: None,
            offset: None,
            movie_id: None,
        };
        let query = GetDiaryQuery::from(params);
        assert!(matches!(query.sort_by, Some(domain::models::SortDirection::Ascending)));
    }

    #[test]
    fn sort_by_other_string_becomes_descending() {
        let params = DiaryQueryParams {
            sort_by: Some("desc".to_string()),
            limit: None,
            offset: None,
            movie_id: None,
        };
        let query = GetDiaryQuery::from(params);
        assert!(matches!(query.sort_by, Some(domain::models::SortDirection::Descending)));
    }

    #[test]
    fn diary_response_serializes_correctly() {
        let resp = DiaryResponse {
            items: vec![],
            total_count: 0,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total_count\":0"));
        assert!(json.contains("\"items\":[]"));
    }

    #[test]
    fn diary_query_params_fields_are_optional() {
        let params = DiaryQueryParams {
            limit: None,
            offset: None,
            sort_by: None,
            movie_id: None,
        };
        assert!(params.limit.is_none());
        assert!(params.sort_by.is_none());
    }

    #[test]
    fn login_request_deserializes() {
        let json = r#"{"email":"a@b.com","password":"secret"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "a@b.com");
    }

    #[test]
    fn form_accepts_date_only() {
        let data = LogReviewData::try_from(make_form("2024-03-15")).unwrap();
        assert_eq!(data.watched_at.format("%H:%M:%S").to_string(), "00:00:00");
        assert_eq!(data.watched_at.format("%Y-%m-%d").to_string(), "2024-03-15");
    }
}
