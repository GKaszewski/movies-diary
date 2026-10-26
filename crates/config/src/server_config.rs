use serde::Deserialize;

use crate::lenient;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    #[serde(deserialize_with = "lenient::number")]
    pub port: u16,
    pub base_url: String,
    pub cors_origins: String,
    #[serde(deserialize_with = "lenient::true_or_one")]
    pub allow_registration: bool,
    #[serde(deserialize_with = "lenient::true_only")]
    pub secure_cookies: bool,
    #[serde(deserialize_with = "lenient::number")]
    pub rate_limit: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
            base_url: "http://localhost:3000".into(),
            cors_origins: String::new(),
            allow_registration: false,
            secure_cookies: false,
            rate_limit: 60,
        }
    }
}
