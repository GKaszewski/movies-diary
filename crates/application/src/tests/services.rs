use crate::services::Services;
use crate::test_helpers::NoopReviewLogger;
use domain::testing::{
    FakeAuthService, FakeDocumentParser, FakeMetadataClient, FakePasswordHasher, FakePosterFetcher,
    NoopEventPublisher, NoopObjectStorage, PanicDiaryExporter,
};
use std::sync::Arc;

#[test]
fn services_clone_shares_the_same_port_instances() {
    let logger: Arc<dyn crate::ports::ReviewLogger> = Arc::new(NoopReviewLogger);
    let s = Services {
        review_logger: Arc::clone(&logger),
        person_enrichment: None,
        auth: Arc::new(FakeAuthService),
        password_hasher: Arc::new(FakePasswordHasher),
        metadata: Arc::new(FakeMetadataClient),
        poster_fetcher: Arc::new(FakePosterFetcher),
        object_storage: Arc::new(NoopObjectStorage),
        event_publisher: NoopEventPublisher::new(),
        diary_exporter: Arc::new(PanicDiaryExporter),
        document_parser: Arc::new(FakeDocumentParser),
    };

    let cloned = s.clone();

    assert!(
        Arc::ptr_eq(&s.review_logger, &cloned.review_logger),
        "Clone must share port instances, not duplicate them"
    );
    assert!(
        Arc::ptr_eq(&s.review_logger, &logger),
        "the field must hold the Arc it was given"
    );
    assert!(
        cloned.person_enrichment.is_none(),
        "optional ports must survive Clone as None"
    );
}
