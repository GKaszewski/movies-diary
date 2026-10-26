use domain::{errors::DomainError, events::DomainEvent};

use crate::search::deps::RequestReindexDeps;

pub async fn execute(deps: &RequestReindexDeps) -> Result<(), DomainError> {
    deps.event_publisher
        .publish(&DomainEvent::SearchReindexRequested)
        .await
}
