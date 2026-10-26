use domain::{
    errors::DomainError,
    events::DomainEvent,
    testing::{NoopEventPublisher, RefusingEventPublisher},
};

use application::search::{deps::RequestReindexDeps, request_reindex};

#[tokio::test]
async fn publishes_a_reindex_request() {
    let publisher = NoopEventPublisher::new();
    let deps = RequestReindexDeps {
        event_publisher: publisher.clone(),
    };

    request_reindex::execute(&deps).await.unwrap();

    assert!(matches!(
        publisher.published().as_slice(),
        [DomainEvent::SearchReindexRequested]
    ));
}

#[tokio::test]
async fn a_publish_failure_is_returned() {
    let deps = RequestReindexDeps {
        event_publisher: std::sync::Arc::new(RefusingEventPublisher),
    };

    let err = request_reindex::execute(&deps).await.unwrap_err();

    assert!(matches!(err, DomainError::InfrastructureError(_)));
}
