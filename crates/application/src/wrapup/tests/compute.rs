use chrono::NaiveDate;
use domain::models::WrapUpMovieRow;
use domain::models::wrapup::{DateRange, WrapUpScope};
use domain::testing::InMemoryWrapUpStatsQuery;
use uuid::Uuid;

use crate::wrapup::queries::ComputeWrapUpQuery;

fn make_row(title: &str, rating: u8, watched_at: &str) -> WrapUpMovieRow {
    WrapUpMovieRow {
        movie_id: Uuid::new_v4(),
        title: title.to_string(),
        release_year: 2024,
        director: Some("Director".to_string()),
        poster_path: None,
        rating,
        watched_at: chrono::NaiveDateTime::parse_from_str(
            &format!("{watched_at} 20:00:00"),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap(),
        user_id: Uuid::new_v4(),
        runtime_minutes: Some(120),
        budget_usd: Some(50_000_000),
        original_language: Some("en".to_string()),
        genres: vec!["Action".to_string()],
        keywords: vec!["heist".to_string()],
        cast_names: vec![("Actor A".to_string(), 1, 12345)],
        cast_profile_paths: vec![None],
    }
}

fn year_2024_range() -> DateRange {
    DateRange::new(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn empty_report() {
    let stats = InMemoryWrapUpStatsQuery::new();
    let user_id = Uuid::new_v4();

    let report = super::execute(
        stats,
        ComputeWrapUpQuery {
            scope: WrapUpScope::User(user_id),
            date_range: year_2024_range(),
        },
    )
    .await
    .unwrap();

    assert_eq!(report.total_movies, 0);
    assert!(report.avg_rating.is_none());
    assert_eq!(report.rating_distribution, [0; 5]);
}

#[tokio::test]
async fn basic_stats() {
    let user_id = Uuid::new_v4();
    let mut r1 = make_row("Film A", 4, "2024-03-10");
    r1.user_id = user_id;
    r1.runtime_minutes = Some(120);
    r1.genres = vec!["Action".to_string()];

    let mut r2 = make_row("Film B", 2, "2024-03-20");
    r2.user_id = user_id;
    r2.runtime_minutes = Some(90);
    r2.genres = vec!["Comedy".to_string()];

    let stats = InMemoryWrapUpStatsQuery::with_rows(vec![r1, r2]);

    let report = super::execute(
        stats,
        ComputeWrapUpQuery {
            scope: WrapUpScope::User(user_id),
            date_range: year_2024_range(),
        },
    )
    .await
    .unwrap();

    assert_eq!(report.total_movies, 2);
    assert_eq!(report.total_watch_time_minutes, 210);
    assert!((report.avg_rating.unwrap() - 3.0).abs() < f64::EPSILON);
    assert_eq!(report.rating_distribution, [0, 1, 0, 1, 0]);
    assert_eq!(report.busiest_month.as_deref(), Some("March 2024"));
    assert_eq!(report.director_diversity, 1);
    assert_eq!(report.genre_diversity, 2);
}

#[tokio::test]
async fn rewatch_detection() {
    let user_id = Uuid::new_v4();
    let movie_id = Uuid::new_v4();

    let mut r1 = make_row("Film A", 3, "2024-02-01");
    r1.user_id = user_id;
    r1.movie_id = movie_id;

    let mut r2 = make_row("Film A", 5, "2024-06-01");
    r2.user_id = user_id;
    r2.movie_id = movie_id;

    let stats = InMemoryWrapUpStatsQuery::with_rows(vec![r1, r2]);

    let report = super::execute(
        stats,
        ComputeWrapUpQuery {
            scope: WrapUpScope::User(user_id),
            date_range: year_2024_range(),
        },
    )
    .await
    .unwrap();

    assert_eq!(report.total_rewatches, 1);
    assert_eq!(
        report.most_rewatched_movie.as_ref().unwrap().title,
        "Film A"
    );
    assert!((report.avg_rating_change_on_rewatch.unwrap() - 2.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn global_scope() {
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let mut r1 = make_row("Film X", 4, "2024-05-01");
    r1.user_id = user_a;

    let mut r2 = make_row("Film Y", 3, "2024-07-01");
    r2.user_id = user_b;

    let stats = InMemoryWrapUpStatsQuery::with_rows(vec![r1, r2]);

    let report = super::execute(
        stats,
        ComputeWrapUpQuery {
            scope: WrapUpScope::Global,
            date_range: year_2024_range(),
        },
    )
    .await
    .unwrap();

    assert_eq!(report.total_movies, 2);
    assert_eq!(report.total_users_active, Some(2));
    assert!(report.most_active_user.is_some());
}
