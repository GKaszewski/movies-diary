use std::fmt;

use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub backend: String,
    pub path: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub bucket: Option<String>,
    pub region: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "local".into(),
            path: "./images".into(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            bucket: None,
            region: "minio".into(),
        }
    }
}

impl fmt::Debug for StorageConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageConfig")
            .field("backend", &self.backend)
            .field("path", &self.path)
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}
