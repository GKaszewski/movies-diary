use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryExporter, DocumentParser, EventPublisher, MetadataClient, ObjectStorage,
    PasswordHasher, PersonEnrichmentClient, PosterFetcherClient,
};

use crate::ports::ReviewLogger;

/// Services the application layer needs, assembled by the composition root.
/// Adapter-typed ports do not belong here — the AP port inversion removed the
/// last of them; see ADR-0008.
#[derive(Clone)]
pub struct Services {
    pub auth: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub metadata: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub review_logger: Arc<dyn ReviewLogger>,
    pub person_enrichment: Option<Arc<dyn PersonEnrichmentClient>>,
}
