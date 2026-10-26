use std::fmt;

use serde::Deserialize;

use crate::lenient;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct MetadataConfig {
    pub tmdb_api_key: Option<String>,
    pub omdb_api_key: Option<String>,
    #[serde(deserialize_with = "lenient::number")]
    pub poster_fetch_timeout_seconds: u64,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key: None,
            omdb_api_key: None,
            poster_fetch_timeout_seconds: 30,
        }
    }
}

impl fmt::Debug for MetadataConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetadataConfig")
            .field(
                "tmdb_api_key",
                &self.tmdb_api_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "omdb_api_key",
                &self.omdb_api_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "poster_fetch_timeout_seconds",
                &self.poster_fetch_timeout_seconds,
            )
            .finish()
    }
}
