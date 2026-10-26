use std::fmt;

use serde::Deserialize;

use crate::lenient;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub jwt_secret: Option<String>,
    #[serde(deserialize_with = "lenient::number")]
    pub jwt_ttl_seconds: u64,
    #[serde(deserialize_with = "lenient::number")]
    pub refresh_ttl_seconds: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: None,
            jwt_ttl_seconds: 900,
            refresh_ttl_seconds: 2_592_000,
        }
    }
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthConfig")
            .field(
                "jwt_secret",
                &self.jwt_secret.as_ref().map(|_| "<redacted>"),
            )
            .field("jwt_ttl_seconds", &self.jwt_ttl_seconds)
            .field("refresh_ttl_seconds", &self.refresh_ttl_seconds)
            .finish()
    }
}
