use std::sync::Arc;

use application::config::AppSettings;

#[derive(Clone)]
pub struct AppContext {
    pub deps: Arc<application::Deps>,
    pub services: application::Services,
    pub config: AppSettings,
    pub instance: domain::value_objects::InstanceIdentity,
    #[cfg(feature = "federation")]
    pub ap_document: Arc<dyn domain::ports::ApDocumentPort>,
    #[cfg(feature = "federation")]
    pub ap_blocklist: Arc<dyn domain::ports::InstanceBlocklistPort>,
}
