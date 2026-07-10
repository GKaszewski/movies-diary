use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::ReviewHistory,
    ports::DiaryQuery,
    services::review_history::{ReviewHistoryAnalyzer, Trend},
    value_objects::MovieId,
};

use crate::diary::queries::GetReviewHistoryQuery;

pub async fn execute(
    diary: &Arc<dyn DiaryQuery>,
    query: GetReviewHistoryQuery,
) -> Result<(ReviewHistory, Trend), DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);

    let mut history = diary.get_review_history(&movie_id).await?;

    let trend = ReviewHistoryAnalyzer::rating_trend(&history)?;

    ReviewHistoryAnalyzer::sort_chronologically(&mut history);

    Ok((history, trend))
}

#[cfg(test)]
#[path = "tests/get_review_history.rs"]
mod tests;
