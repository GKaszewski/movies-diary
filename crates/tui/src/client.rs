use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── DTOs (mirror backend dtos.rs exactly) ────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogReviewRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_metadata_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_release_year: Option<u16>,
    pub rating: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub watched_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiaryResponse {
    pub items: Vec<DiaryEntryDto>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiaryEntryDto {
    pub movie: MovieDto,
    pub review: ReviewDto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MovieDto {
    pub id: Uuid,
    pub title: String,
    pub release_year: u16,
    pub director: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewDto {
    pub id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewHistoryResponse {
    pub movie: MovieDto,
    pub viewings: Vec<ReviewDto>,
    pub trend: String,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("server error {status}: {body}")]
    Unknown { status: u16, body: String },
}

async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.map_err(ApiError::Network)?;
    Err(match status.as_u16() {
        401 => ApiError::Unauthorized,
        403 => ApiError::Forbidden,
        404 => ApiError::NotFound,
        400 => ApiError::Validation(body),
        code => ApiError::Unknown { status: code, body },
    })
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct ApiClient {
    base_url: std::sync::RwLock<String>,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(url: &str) -> Self {
        Self {
            base_url: std::sync::RwLock::new(url.to_string()),
            http: reqwest::Client::new(),
        }
    }

    pub fn update_url(&self, url: &str) {
        *self.base_url.write().unwrap() = url.to_string();
    }

    fn url(&self) -> String {
        self.base_url.read().unwrap().clone()
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, ApiError> {
        let resp = self
            .http
            .post(format!("{}/api/auth/login", self.url()))
            .json(&LoginRequest {
                email: email.into(),
                password: password.into(),
            })
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn get_diary(
        &self,
        token: &str,
        offset: u32,
        limit: u32,
    ) -> Result<DiaryResponse, ApiError> {
        let resp = self
            .http
            .get(format!("{}/api/diary", self.url()))
            .query(&[("offset", offset), ("limit", limit)])
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn get_movie_history(
        &self,
        token: &str,
        movie_id: Uuid,
    ) -> Result<ReviewHistoryResponse, ApiError> {
        let resp = self
            .http
            .get(format!("{}/api/movies/{}/history", self.url(), movie_id))
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn create_review(&self, token: &str, req: &LogReviewRequest) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(format!("{}/api/reviews", self.url()))
            .bearer_auth(token)
            .json(req)
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    pub async fn delete_review(&self, token: &str, review_id: Uuid) -> Result<(), ApiError> {
        let resp = self
            .http
            .delete(format!("{}/api/reviews/{}", self.url(), review_id))
            .bearer_auth(token)
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apierror_unauthorized_display() {
        let err = ApiError::Unauthorized;
        assert!(matches!(err, ApiError::Unauthorized));
        assert_eq!(err.to_string(), "unauthorized");
    }

    #[test]
    fn apierror_validation_display() {
        let err = ApiError::Validation("rating must be 0-5".into());
        assert!(err.to_string().contains("validation error"));
    }

    #[test]
    fn log_review_request_skips_none_fields() {
        let req = LogReviewRequest {
            external_metadata_id: None,
            manual_title: Some("The Matrix".into()),
            manual_release_year: None,
            rating: 5,
            comment: None,
            watched_at: "2024-01-15T20:00:00".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("external_metadata_id"));
        assert!(!json.contains("manual_release_year"));
        assert!(json.contains("\"manual_title\":\"The Matrix\""));
        assert!(json.contains("\"rating\":5"));
    }

    #[test]
    fn api_client_update_url() {
        let client = ApiClient::new("http://localhost:3000");
        assert!(client.url().contains("3000"));
        client.update_url("http://localhost:8080");
        assert!(client.url().contains("8080"));
    }
}
