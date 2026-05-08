use std::sync::Arc;

use domain::ports::UserRepository;

use crate::repository::FederationRepository;

#[derive(Clone)]
pub struct FederationData {
    pub(crate) federation_repo: Arc<dyn FederationRepository>,
    pub(crate) user_repo: Arc<dyn UserRepository>,
    pub(crate) base_url: String,
    pub(crate) domain: String,
}

impl FederationData {
    pub fn new(
        federation_repo: Arc<dyn FederationRepository>,
        user_repo: Arc<dyn UserRepository>,
        base_url: String,
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
            base_url,
            domain,
        }
    }
}
