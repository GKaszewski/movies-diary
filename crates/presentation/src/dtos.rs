use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DiaryQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub movie_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct LogReviewForm {
    pub external_metadata_id: Option<String>,
    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
