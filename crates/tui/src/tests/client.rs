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
        manual_director: None,
        rating: 5,
        comment: None,
        watched_at: "2024-01-15T20:00:00".into(),
        watch_medium: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("external_metadata_id"));
    assert!(!json.contains("manual_release_year"));
    assert!(!json.contains("manual_director"));
    assert!(json.contains("\"manual_title\":\"The Matrix\""));
    assert!(json.contains("\"rating\":5"));
}

#[test]
fn log_review_request_includes_director_when_set() {
    let req = LogReviewRequest {
        external_metadata_id: None,
        manual_title: Some("Dune".into()),
        manual_release_year: Some(2021),
        manual_director: Some("Denis Villeneuve".into()),
        rating: 5,
        comment: None,
        watched_at: "2024-01-15T20:00:00".into(),
        watch_medium: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"manual_director\":\"Denis Villeneuve\""));
}

#[test]
fn api_client_builds_versioned_urls() {
    let client = ApiClient::new("http://localhost:3000");
    assert_eq!(client.api("/diary"), "http://localhost:3000/api/v1/diary");
    assert_eq!(
        client.api("/auth/login"),
        "http://localhost:3000/api/v1/auth/login"
    );
    assert_eq!(
        client.api("/social/follow"),
        "http://localhost:3000/api/v1/social/follow"
    );
}

#[test]
fn api_client_update_url() {
    let client = ApiClient::new("http://localhost:3000");
    assert!(client.url().contains("3000"));
    client.update_url("http://localhost:8080");
    assert!(client.url().contains("8080"));
    assert_eq!(client.api("/diary"), "http://localhost:8080/api/v1/diary");
}
