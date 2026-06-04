use anyhow::Context;
use object_store::{ObjectStore, aws::AmazonS3Builder, local::LocalFileSystem};
use std::sync::Arc;

pub struct StorageConfig(Arc<dyn ObjectStore>);

impl StorageConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let backend = std::env::var("IMAGE_STORAGE_BACKEND").unwrap_or_else(|_| "local".into());

        let store: Arc<dyn ObjectStore> = match backend.as_str() {
            "s3" => build_s3_store(
                &std::env::var("MINIO_ENDPOINT").context("MINIO_ENDPOINT required")?,
                &std::env::var("MINIO_ACCESS_KEY_ID").context("MINIO_ACCESS_KEY_ID required")?,
                &std::env::var("MINIO_SECRET_ACCESS_KEY")
                    .context("MINIO_SECRET_ACCESS_KEY required")?,
                &std::env::var("MINIO_BUCKET").context("MINIO_BUCKET required")?,
                &std::env::var("MINIO_REGION").unwrap_or_else(|_| "minio".to_string()),
            )?,
            "local" => build_local_store(
                &std::env::var("IMAGE_STORAGE_PATH").unwrap_or_else(|_| "./images".into()),
            )?,
            other => {
                anyhow::bail!("Unknown IMAGE_STORAGE_BACKEND: {other:?}. Valid values: s3, local")
            }
        };

        Ok(Self(store))
    }

    pub fn build_store(self) -> Arc<dyn ObjectStore> {
        self.0
    }
}

fn build_s3_store(
    endpoint: &str,
    access_key_id: &str,
    secret_access_key: &str,
    bucket: &str,
    region: &str,
) -> anyhow::Result<Arc<dyn ObjectStore>> {
    let store = AmazonS3Builder::new()
        .with_endpoint(endpoint)
        .with_access_key_id(access_key_id)
        .with_secret_access_key(secret_access_key)
        .with_bucket_name(bucket)
        .with_region(region)
        .with_allow_http(true)
        .build()
        .context("Failed to build S3/Minio store")?;
    Ok(Arc::new(store))
}

fn build_local_store(path: &str) -> anyhow::Result<Arc<dyn ObjectStore>> {
    std::fs::create_dir_all(path).context("Failed to create image storage directory")?;
    let store = LocalFileSystem::new_with_prefix(path)
        .context("Failed to initialise local file system store")?;
    Ok(Arc::new(store))
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
