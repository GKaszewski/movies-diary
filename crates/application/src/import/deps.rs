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

pub struct GetMappingStageDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
}

pub struct GetPreviewStageDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
}

pub struct GetSessionStateDeps {
    pub import_session: Arc<dyn ImportSessionRepository>,
}

pub struct DeleteImportProfileDeps {
    pub import_profile: Arc<dyn ImportProfileRepository>,
}

pub struct ListImportProfilesDeps {
    pub import_profile: Arc<dyn ImportProfileRepository>,
}

/// Backs `apply_profile_and_map`, which internally drives `apply_profile::execute`
/// then `apply_mapping::execute` — these fields are exactly the union of
/// `ApplyProfileDeps` and `ApplyMappingDeps`'s fields, cloned once here and used to
/// build each nested deps struct inline at the call site (see that file's doc
/// comment for why: no use-case signature changes, per this task's constraints).
pub struct ApplyProfileAndMapDeps {
    pub import_profile: Arc<dyn ImportProfileRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub movie_query: Arc<dyn MovieQuery>,
}
