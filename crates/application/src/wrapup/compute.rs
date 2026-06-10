use crate::context::AppContext;
use crate::wrapup::queries::ComputeWrapUpQuery;
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use domain::services::wrapup_analyzer;

pub async fn execute(
    ctx: &AppContext,
    query: ComputeWrapUpQuery,
) -> Result<WrapUpReport, DomainError> {
    let rows = ctx
        .repos
        .wrapup_stats
        .get_reviews_with_profiles(&query.scope, &query.date_range)
        .await?;
    Ok(wrapup_analyzer::build_report(query.scope, query.date_range, &rows))
}

#[cfg(test)]
#[path = "tests/compute.rs"]
mod tests;
