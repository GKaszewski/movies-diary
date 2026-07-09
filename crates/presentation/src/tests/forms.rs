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
        watch_medium: None,
        csrf_token: String::new(),
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
        watch_medium: None,
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
fn api_accepts_datetime_without_seconds() {
    let data = LogReviewData::try_from(make_request("2024-03-15T20:30")).unwrap();
    assert_eq!(data.watched_at.format("%H:%M").to_string(), "20:30");
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
fn sort_by_asc_string_becomes_ascending() {
    let params = DiaryQueryParams {
        sort_by: Some("asc".to_string()),
        limit: None,
        offset: None,
        movie_id: None,
    };
    let query = to_diary_query(params);
    assert!(matches!(
        query.sort_by,
        Some(domain::models::SortDirection::Ascending)
    ));
}

#[test]
fn sort_by_other_string_becomes_descending() {
    let params = DiaryQueryParams {
        sort_by: Some("desc".to_string()),
        limit: None,
        offset: None,
        movie_id: None,
    };
    let query = to_diary_query(params);
    assert!(matches!(
        query.sort_by,
        Some(domain::models::SortDirection::Descending)
    ));
}

#[test]
fn form_accepts_date_only() {
    let data = LogReviewData::try_from(make_form("2024-03-15")).unwrap();
    assert_eq!(data.watched_at.format("%H:%M:%S").to_string(), "00:00:00");
    assert_eq!(data.watched_at.format("%Y-%m-%d").to_string(), "2024-03-15");
}
