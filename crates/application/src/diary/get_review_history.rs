use domain::{
    errors::DomainError,
    models::ReviewHistory,
    services::review_history::{ReviewHistoryAnalyzer, Trend},
    value_objects::MovieId,
};

use crate::diary::deps::GetReviewHistoryDeps;
use crate::diary::queries::GetReviewHistoryQuery;

pub async fn execute(
    deps: &GetReviewHistoryDeps,
    query: GetReviewHistoryQuery,
) -> Result<(ReviewHistory, Trend), DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);

    let mut history = deps.diary.get_review_history(&movie_id).await?;

    let trend = ReviewHistoryAnalyzer::rating_trend(&history)?;

    ReviewHistoryAnalyzer::sort_chronologically(&mut history);

    Ok((history, trend))
}

#[cfg(test)]
#[path = "tests/get_review_history.rs"]
mod tests;
