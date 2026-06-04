mod config;
pub use config::StorageConfig;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, ObjectStorage},
};
use futures::StreamExt;
use object_store::{ObjectStore, path::Path};
use std::sync::Arc;

pub struct ObjectStorageAdapter {
    store: Arc<dyn ObjectStore>,
}

impl ObjectStorageAdapter {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    pub fn from_config(config: StorageConfig) -> Self {
        Self::new(config.build_store())
    }

    async fn get_exact(&self, key: &str) -> Result<Vec<u8>, DomainError> {
        let path = Path::from(key);
        let result = self.store.get(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => DomainError::NotFound("Image not found".into()),
            _ => DomainError::InfrastructureError(e.to_string()),
        })?;
        result
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

#[async_trait]
impl ObjectStorage for ObjectStorageAdapter {
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError> {
        let path = Path::from(key);
        self.store
            .put(&path, image_bytes.to_vec().into())
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(key.to_string())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError> {
        match self.get_exact(key).await {
            Ok(bytes) => return Ok(bytes),
            Err(DomainError::NotFound(_)) if !has_image_ext(key) => {}
            Err(e) => return Err(e),
        }
        // Key may reference a pre-conversion path; try converted extensions.
        for ext in [".webp", ".avif"] {
            let candidate = format!("{key}{ext}");
            if let Ok(bytes) = self.get_exact(&candidate).await {
                return Ok(bytes);
            }
        }
        Err(DomainError::NotFound("Image not found".into()))
    }

    async fn get_stream(
        &self,
        key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        let path = Path::from(key);
        let result = self.store.get(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => DomainError::NotFound("not found".into()),
            _ => DomainError::InfrastructureError(e.to_string()),
        })?;
        let stream = result.into_stream().map(|chunk| {
            chunk
                .map(|b| bytes::Bytes::from(b.to_vec()))
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))
        });
        Ok(Box::pin(stream))
    }

    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        let path = Path::from(key);
        match self.store.delete(&path).await {
            Ok(()) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(DomainError::InfrastructureError(e.to_string())),
        }
    }
}

fn has_image_ext(key: &str) -> bool {
    key.ends_with(".webp")
        || key.ends_with(".avif")
        || key.ends_with(".png")
        || key.ends_with(".jpg")
        || key.ends_with(".jpeg")
}

pub struct ImageCleanupHandler {
    object_storage: Arc<dyn ObjectStorage>,
}

impl ImageCleanupHandler {
    pub fn new(object_storage: Arc<dyn ObjectStorage>) -> Self {
        Self { object_storage }
    }
}

#[async_trait]
impl EventHandler for ImageCleanupHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let poster_path = match event {
            DomainEvent::MovieDeleted { poster_path, .. } => poster_path,
            _ => return Ok(()),
        };
        let Some(path) = poster_path else {
            return Ok(());
        };
        if let Err(e) = self.object_storage.delete(path.value()).await {
            tracing::warn!("image cleanup failed for {}: {e}", path.value());
        }
        Ok(())
    }
}

pub fn create() -> anyhow::Result<Arc<dyn ObjectStorage>> {
    Ok(Arc::new(ObjectStorageAdapter::from_config(
        StorageConfig::from_env()?,
    )))
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
