use crate::{errors::DomainError, models::ReviewHistory, value_objects::Rating};

pub struct ReviewHistoryAnalyzer;

#[derive(Debug, PartialEq)]
pub enum Trend {
    Improved,
    Declined,
    Neutral,
}

impl ReviewHistoryAnalyzer {
    pub fn sort_chronologically(history: &mut ReviewHistory) {
        history.sort_by_date();
    }

    pub fn get_latest_rating(history: &ReviewHistory) -> Option<&Rating> {
        history
            .viewings()
            .iter()
            .max_by_key(|r| r.watched_at())
            .map(|r| r.rating())
    }

    pub fn rating_trend(history: &ReviewHistory) -> Result<Trend, DomainError> {
        if history.viewings().len() < 2 {
            return Ok(Trend::Neutral);
        }

        let latest_review = history
            .viewings()
            .iter()
            .max_by_key(|r| r.watched_at())
            .unwrap();
        let latest_rating = latest_review.rating().value() as f32;

        let count = history.viewings().len() as f32;
        let total: f32 = history
            .viewings()
            .iter()
            .map(|r| r.rating().value() as f32)
            .sum();
        let historical_average = total / count;

        if latest_rating > historical_average {
            Ok(Trend::Improved)
        } else if latest_rating < historical_average {
            Ok(Trend::Declined)
        } else {
            Ok(Trend::Neutral)
        }
    }
}

#[cfg(test)]
mod tests {
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
}
