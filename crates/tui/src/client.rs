use api_types::{
    ActorListResponse, ActorUrlRequest, DiaryResponse, FollowRequest, LogReviewRequest,
    LoginRequest, LoginResponse, ReviewHistoryResponse,
};
use uuid::Uuid;

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

    fn api(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.url(), path)
    }

    // ── Auth ──────────────────────────────────────────────────────────────────

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, ApiError> {
        let resp = self
            .http
            .post(self.api("/auth/login"))
            .json(&LoginRequest {
                email: email.into(),
                password: password.into(),
            })
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    // ── Diary ─────────────────────────────────────────────────────────────────

    pub async fn get_diary(
        &self,
        token: &str,
        offset: u32,
        limit: u32,
    ) -> Result<DiaryResponse, ApiError> {
        let resp = self
            .http
            .get(self.api("/diary"))
            .query(&[("offset", offset), ("limit", limit)])
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn export_diary(
        &self,
        token: &str,
        format: &str,
    ) -> Result<Vec<u8>, ApiError> {
        let resp = self
            .http
            .get(self.api("/diary/export"))
            .query(&[("format", format)])
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.bytes().await?.to_vec())
    }

    // ── Reviews ───────────────────────────────────────────────────────────────

    pub async fn get_movie_history(
        &self,
        token: &str,
        movie_id: Uuid,
    ) -> Result<ReviewHistoryResponse, ApiError> {
        let resp = self
            .http
            .get(self.api(&format!("/movies/{movie_id}/history")))
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn create_review(&self, token: &str, req: &LogReviewRequest) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/reviews"))
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
            .delete(self.api(&format!("/reviews/{review_id}")))
            .bearer_auth(token)
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    // ── Social (ActivityPub) ──────────────────────────────────────────────────

    pub async fn get_following(&self, token: &str) -> Result<ActorListResponse, ApiError> {
        let resp = self
            .http
            .get(self.api("/social/following"))
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn get_followers(&self, token: &str) -> Result<ActorListResponse, ApiError> {
        let resp = self
            .http
            .get(self.api("/social/followers"))
            .bearer_auth(token)
            .send()
            .await?;
        Ok(check_status(resp).await?.json().await?)
    }

    pub async fn follow(&self, token: &str, handle: &str) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/social/follow"))
            .bearer_auth(token)
            .json(&FollowRequest { handle: handle.into() })
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    pub async fn unfollow(&self, token: &str, actor_url: &str) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/social/unfollow"))
            .bearer_auth(token)
            .json(&ActorUrlRequest { actor_url: actor_url.into() })
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    pub async fn accept_follower(&self, token: &str, actor_url: &str) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/social/followers/accept"))
            .bearer_auth(token)
            .json(&ActorUrlRequest { actor_url: actor_url.into() })
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    pub async fn reject_follower(&self, token: &str, actor_url: &str) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/social/followers/reject"))
            .bearer_auth(token)
            .json(&ActorUrlRequest { actor_url: actor_url.into() })
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }

    pub async fn remove_follower(&self, token: &str, actor_url: &str) -> Result<(), ApiError> {
        let resp = self
            .http
            .post(self.api("/social/followers/remove"))
            .bearer_auth(token)
            .json(&ActorUrlRequest { actor_url: actor_url.into() })
            .send()
            .await?;
        check_status(resp).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/client.rs"]
mod tests;
