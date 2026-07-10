use std::sync::Arc;

use chrono::Duration;
use domain::{errors::DomainError, ports::WatchEventCommand};

pub async fn execute(watch_event_command: Arc<dyn WatchEventCommand>) -> Result<u64, DomainError> {
    let cutoff = chrono::Utc::now().naive_utc() - Duration::days(30);
    watch_event_command.delete_non_pending_older_than(cutoff).await
}

#[cfg(test)]
#[path = "tests/cleanup.rs"]
mod tests;
