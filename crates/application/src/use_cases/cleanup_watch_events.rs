use chrono::Duration;
use domain::errors::DomainError;

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext) -> Result<u64, DomainError> {
    let cutoff = chrono::Utc::now().naive_utc() - Duration::days(30);
    ctx.watch_event_repository
        .delete_non_pending_older_than(cutoff)
        .await
}
