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
        history
            .viewings
            .sort_by(|a, b| a.watched_at().cmp(&b.watched_at()));
    }

    pub fn get_latest_rating(history: &ReviewHistory) -> Option<&Rating> {
        history
            .viewings
            .iter()
            .max_by_key(|r| r.watched_at())
            .map(|r| r.rating())
    }

    pub fn rating_trend(history: &ReviewHistory) -> Result<Trend, DomainError> {
        if history.viewings.len() < 2 {
            return Ok(Trend::Neutral);
        }

        let mut sorted_history = history.clone();
        Self::sort_chronologically(&mut sorted_history);

        let latest_review = sorted_history.viewings.pop().unwrap();
        let latest_rating = latest_review.rating().value() as f32;

        let previous_sum: u32 = sorted_history
            .viewings
            .iter()
            .map(|r| r.rating().value() as u32)
            .sum();
        let historical_average = previous_sum as f32 / sorted_history.viewings.len() as f32;

        if latest_rating > historical_average {
            Ok(Trend::Improved)
        } else if latest_rating < historical_average {
            Ok(Trend::Declined)
        } else {
            Ok(Trend::Neutral)
        }
    }
}
