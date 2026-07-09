use super::*;
use crate::models::{Movie, Review, ReviewHistory};
use crate::value_objects::{MovieId, MovieTitle, Rating, ReleaseYear, UserId};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

fn make_movie() -> Movie {
    Movie::new(
        None,
        MovieTitle::new("Test".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    )
}

fn dt(year: i32, month: u32, day: u32) -> NaiveDateTime {
    NaiveDateTime::new(
        NaiveDate::from_ymd_opt(year, month, day).unwrap(),
        NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    )
}

fn review_with_rating(movie_id: &MovieId, rating: u8, watched_at: NaiveDateTime) -> Review {
    let user_id = UserId::generate();
    Review::new(
        movie_id.clone(),
        user_id,
        Rating::new(rating).unwrap(),
        None,
        watched_at,
        None,
    )
    .unwrap()
}

#[test]
fn neutral_when_empty() {
    let movie = make_movie();
    let history = ReviewHistory::new(movie, vec![]);
    let trend = ReviewHistoryAnalyzer::rating_trend(&history).unwrap();
    assert_eq!(trend, Trend::Neutral);
}

#[test]
fn neutral_when_single_review() {
    let movie = make_movie();
    let r = review_with_rating(movie.id(), 4, dt(2024, 1, 1));
    let history = ReviewHistory::new(movie, vec![r]);
    let trend = ReviewHistoryAnalyzer::rating_trend(&history).unwrap();
    assert_eq!(trend, Trend::Neutral);
}

#[test]
fn improved_when_latest_above_average() {
    let movie = make_movie();
    let viewings = vec![
        review_with_rating(movie.id(), 2, dt(2024, 1, 1)),
        review_with_rating(movie.id(), 3, dt(2024, 2, 1)),
        review_with_rating(movie.id(), 5, dt(2024, 3, 1)),
    ];
    let history = ReviewHistory::new(movie, viewings);
    let trend = ReviewHistoryAnalyzer::rating_trend(&history).unwrap();
    assert_eq!(trend, Trend::Improved);
}

#[test]
fn declined_when_latest_below_average() {
    let movie = make_movie();
    let viewings = vec![
        review_with_rating(movie.id(), 5, dt(2024, 1, 1)),
        review_with_rating(movie.id(), 4, dt(2024, 2, 1)),
        review_with_rating(movie.id(), 2, dt(2024, 3, 1)),
    ];
    let history = ReviewHistory::new(movie, viewings);
    let trend = ReviewHistoryAnalyzer::rating_trend(&history).unwrap();
    assert_eq!(trend, Trend::Declined);
}
