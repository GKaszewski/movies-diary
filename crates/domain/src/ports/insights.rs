use async_trait::async_trait;

use crate::{errors::DomainError, models::insights::WatchRow, value_objects::UserId};

/// Read side of Insights (ADR-0010): fetches one denormalized `WatchRow` per
/// review across the user's whole diary. All thresholds and baseline
/// comparisons live in the pure `insights_analyzer`, never in adapters —
/// SQL's job here is "rows in, rows out" and nothing else.
#[async_trait]
pub trait InsightsQuery: Send + Sync {
    async fn get_watch_rows(&self, user_id: &UserId) -> Result<Vec<WatchRow>, DomainError>;
}
