use std::sync::Arc;

use chrono::Duration;
use domain::{errors::DomainError, ports::WatchEventRepository};

pub async fn execute(watch_event: Arc<dyn WatchEventRepository>) -> Result<u64, DomainError> {
    let cutoff = chrono::Utc::now().naive_utc() - Duration::days(30);
    watch_event.delete_non_pending_older_than(cutoff).await
}

#[cfg(test)]
#[path = "tests/cleanup.rs"]
mod tests;
