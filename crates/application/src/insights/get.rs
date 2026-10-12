//! Live Insights for the requesting user (ADR-0010): fetch the whole diary as
//! `WatchRow`s, run the pure analyzer, hand back whatever clears the honesty
//! thresholds. No caching, no snapshots — the same on-the-fly shape as the
//! existing live stats.

use chrono::TimeZone;
use domain::{errors::DomainError, models::Insight, value_objects::UserId};

use crate::insights::deps::GetInsightsDeps;

pub async fn execute<Tz: TimeZone + Sync>(
    deps: &GetInsightsDeps,
    user_id: UserId,
    tz: &Tz,
) -> Result<Vec<Insight>, DomainError> {
    let rows = deps.insights.get_watch_rows(&user_id).await?;
    Ok(domain::services::insights_analyzer::build_insights(
        &rows, tz,
    ))
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
