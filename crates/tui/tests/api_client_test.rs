use api_types::LogReviewRequest;
use tui::client::ApiError;

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
        spoilers: false,
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
        spoilers: false,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"manual_director\":\"Denis Villeneuve\""));
}
