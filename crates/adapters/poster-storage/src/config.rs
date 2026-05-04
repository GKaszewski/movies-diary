use anyhow::Context;
use object_store::{aws::AmazonS3Builder, ObjectStore};
use std::sync::Arc;

pub struct StorageConfig {
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
    region: String,
}

impl StorageConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            endpoint: std::env::var("MINIO_ENDPOINT").context("MINIO_ENDPOINT required")?,
            access_key_id: std::env::var("MINIO_ACCESS_KEY_ID")
                .context("MINIO_ACCESS_KEY_ID required")?,
            secret_access_key: std::env::var("MINIO_SECRET_ACCESS_KEY")
                .context("MINIO_SECRET_ACCESS_KEY required")?,
            bucket: std::env::var("MINIO_BUCKET").context("MINIO_BUCKET required")?,
            region: std::env::var("MINIO_REGION").unwrap_or_else(|_| "minio".to_string()),
        })
    }

    pub fn build_store(self) -> anyhow::Result<Arc<dyn ObjectStore>> {
        let store = AmazonS3Builder::new()
            .with_endpoint(self.endpoint)
            .with_access_key_id(self.access_key_id)
            .with_secret_access_key(self.secret_access_key)
            .with_bucket_name(self.bucket)
            .with_region(self.region)
            .with_allow_http(true)
            .build()
            .context("Failed to build S3/Minio store")?;
        Ok(Arc::new(store))
    }
}
