use chrono::NaiveDate;
use uuid::Uuid;

use crate::models::wrapup::{DateRange, WrapUpScope};
use crate::ports::WrapUpMovieRow;

use super::super::wrapup_analyzer::build_report;

fn row(title: &str, rating: u8, ym: &str) -> WrapUpMovieRow {
    WrapUpMovieRow {
        movie_id: Uuid::new_v4(),
        title: title.to_string(),
        release_year: 2024,
        director: Some("Dir".to_string()),
        poster_path: None,
        rating,
        watched_at: chrono::NaiveDateTime::parse_from_str(
            &format!("{ym}-01 12:00:00"),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap(),
        user_id: Uuid::new_v4(),
        runtime_minutes: Some(100),
        budget_usd: None,
        original_language: Some("en".to_string()),
        genres: vec!["Action".to_string()],
        keywords: vec![],
        cast_names: vec![],
        cast_profile_paths: vec![],
    }
}

fn range() -> DateRange {
    DateRange::new(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    )
    .unwrap()
}

#[test]
fn empty_rows_returns_zero_totals() {
    let report = build_report(WrapUpScope::Global, range(), &[]);
    assert_eq!(report.total_movies, 0);
    assert!(report.avg_rating.is_none());
    assert_eq!(report.rating_distribution, [0u32; 5]);
}

#[test]
fn total_movies_matches_row_count() {
    let rows = vec![row("A", 5, "2024-01"), row("B", 3, "2024-02")];
    let report = build_report(WrapUpScope::Global, range(), &rows);
    assert_eq!(report.total_movies, 2);
}

#[test]
fn avg_rating_is_correct() {
    let rows = vec![row("A", 4, "2024-01"), row("B", 2, "2024-01")];
    let report = build_report(WrapUpScope::Global, range(), &rows);
    let avg = report.avg_rating.unwrap();
    assert!((avg - 3.0).abs() < f64::EPSILON);
}

#[test]
fn rating_distribution_counts_correctly() {
    let rows = vec![
        row("A", 5, "2024-01"),
        row("B", 5, "2024-02"),
        row("C", 3, "2024-03"),
    ];
    let report = build_report(WrapUpScope::Global, range(), &rows);
    assert_eq!(report.rating_distribution[4], 2);
    assert_eq!(report.rating_distribution[2], 1);
}

#[test]
fn movies_per_month_sorted_chronologically() {
    let rows = vec![
        row("A", 3, "2024-03"),
        row("B", 3, "2024-01"),
        row("C", 3, "2024-02"),
    ];
    let report = build_report(WrapUpScope::Global, range(), &rows);
    let yms: Vec<&str> = report
        .movies_per_month
        .iter()
        .map(|m| m.year_month.as_str())
        .collect();
    assert_eq!(yms, ["2024-01", "2024-02", "2024-03"]);
}

#[test]
fn global_scope_populates_global_fields() {
    let rows = vec![row("A", 4, "2024-01")];
    let report = build_report(WrapUpScope::Global, range(), &rows);
    assert!(report.total_users_active.is_some());
}

#[test]
fn user_scope_leaves_global_fields_empty() {
    let rows = vec![row("A", 4, "2024-01")];
    let report = build_report(WrapUpScope::User(Uuid::new_v4()), range(), &rows);
    assert!(report.total_users_active.is_none());
    assert!(report.most_active_user.is_none());
}
