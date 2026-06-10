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
#[path = "tests/review_history.rs"]
mod tests;
