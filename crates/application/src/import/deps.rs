use std::sync::Arc;

use domain::ports::{DocumentParser, ImportProfileRepository, ImportSessionRepository, MovieQuery};

use crate::ports::ReviewLogger;

pub struct CreateSessionDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub document_parser: Arc<dyn DocumentParser>,
}

pub struct ApplyMappingDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub movie_query: Arc<dyn MovieQuery>,
}

pub struct ApplyProfileDeps {
    pub import_profile: Arc<dyn ImportProfileRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
}

pub struct ExecuteImportDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub review_logger: Arc<dyn ReviewLogger>,
}

pub struct SaveProfileDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub import_profile: Arc<dyn ImportProfileRepository>,
}
