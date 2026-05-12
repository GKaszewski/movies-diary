use std::sync::Arc;

use crate::content::ApObjectHandler;
use crate::repository::FederationRepository;
use crate::user::ApUserRepository;
use domain::ports::EventPublisher;

#[derive(Clone)]
pub struct FederationData {
    pub(crate) federation_repo: Arc<dyn FederationRepository>,
    pub(crate) user_repo: Arc<dyn ApUserRepository>,
    pub(crate) object_handler: Arc<dyn ApObjectHandler>,
    pub(crate) base_url: String,
    pub(crate) domain: String,
    pub(crate) allow_registration: bool,
    pub(crate) software_name: String,
    pub(crate) event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl FederationData {
    pub fn new(
        federation_repo: Arc<dyn FederationRepository>,
        user_repo: Arc<dyn ApUserRepository>,
        object_handler: Arc<dyn ApObjectHandler>,
        base_url: String,
        allow_registration: bool,
        software_name: String,
        event_publisher: Option<Arc<dyn EventPublisher>>,
    ) -> Self {
        let domain = base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        Self {
            federation_repo,
            user_repo,
            object_handler,
            base_url,
            domain,
            allow_registration,
            software_name,
            event_publisher,
        }
    }
}
