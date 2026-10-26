use serde::Deserialize;

use crate::{
    AuthConfig, DatabaseConfig, EventsConfig, FederationConfig, ImageConversionConfig,
    MetadataConfig, NatsConfig, ServerConfig, StorageConfig, WrapUpConfig,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub events: EventsConfig,
    #[serde(default)]
    pub nats: NatsConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub image_conversion: ImageConversionConfig,
    #[serde(default)]
    pub metadata: MetadataConfig,
    #[serde(default)]
    pub federation: FederationConfig,
    #[serde(default)]
    pub wrapup: WrapUpConfig,
}
