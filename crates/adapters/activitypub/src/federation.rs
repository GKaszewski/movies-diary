use activitypub_federation::config::{Data, FederationConfig, FederationMiddleware, UrlVerifier};
use activitypub_federation::error::Error as FedError;
use url::Url;

use crate::data::FederationData;

// In debug mode, allow all URLs (including http://localhost:3000 where the
// port colon would otherwise fail the default domain character check).
#[derive(Clone)]
struct PermissiveVerifier;

#[async_trait::async_trait]
impl UrlVerifier for PermissiveVerifier {
    async fn verify(&self, _url: &Url) -> Result<(), FedError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ApFederationConfig(pub FederationConfig<FederationData>);

impl ApFederationConfig {
    pub async fn new(data: FederationData, debug: bool) -> anyhow::Result<Self> {
        let config = if debug {
            FederationConfig::builder()
                .domain(&data.domain)
                .app_data(data)
                .debug(true)
                .http_signature_compat(true)
                .url_verifier(Box::new(PermissiveVerifier))
                .build()
                .await?
        } else {
            FederationConfig::builder()
                .domain(&data.domain)
                .app_data(data)
                .debug(false)
                .http_signature_compat(true)
                .build()
                .await?
        };
        Ok(Self(config))
    }

    pub fn to_request_data(&self) -> Data<FederationData> {
        self.0.to_request_data()
    }

    pub fn middleware(&self) -> FederationMiddleware<FederationData> {
        FederationMiddleware::new(self.0.clone())
    }
}
