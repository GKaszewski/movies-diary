use domain::{
    errors::DomainError,
    models::ReviewHistory,
    services::review_history::{ReviewHistoryAnalyzer, Trend},
    value_objects::MovieId,
};

use crate::{context::AppContext, diary::queries::GetReviewHistoryQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetReviewHistoryQuery,
) -> Result<(ReviewHistory, Trend), DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);

    let mut history = ctx.repos.diary.get_review_history(&movie_id).await?;

    let trend = ReviewHistoryAnalyzer::rating_trend(&history)?;

    ReviewHistoryAnalyzer::sort_chronologically(&mut history);

    Ok((history, trend))
}
