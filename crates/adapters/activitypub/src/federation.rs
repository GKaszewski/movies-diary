use activitypub_federation::config::{Data, FederationConfig, FederationMiddleware};

use crate::data::FederationData;

#[derive(Clone)]
pub struct ApFederationConfig(pub FederationConfig<FederationData>);

impl ApFederationConfig {
    pub async fn new(data: FederationData, debug: bool) -> anyhow::Result<Self> {
        let config = FederationConfig::builder()
            .domain(&data.domain)
            .app_data(data)
            .debug(debug)
            .build()
            .await?;
        Ok(Self(config))
    }

    pub fn to_request_data(&self) -> Data<FederationData> {
        self.0.to_request_data()
    }

    pub fn middleware(&self) -> FederationMiddleware<FederationData> {
        FederationMiddleware::new(self.0.clone())
    }
}
