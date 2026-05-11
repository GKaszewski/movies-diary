mod config;
pub use config::StorageConfig;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, ImageStorage},
};
use object_store::{Attribute, Attributes, ObjectStore, PutOptions, path::Path};
use std::sync::Arc;

fn detect_mime(bytes: &[u8]) -> &'static str {
    infer::get(bytes)
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream")
}

pub struct ImageStorageAdapter {
    store: Arc<dyn ObjectStore>,
}

impl ImageStorageAdapter {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    pub fn from_config(config: StorageConfig) -> Self {
        Self::new(config.build_store())
    }
}

#[async_trait]
impl ImageStorage for ImageStorageAdapter {
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError> {
        let path = Path::from(key);
        let mime = detect_mime(image_bytes);
        let mut attributes = Attributes::new();
        attributes.insert(Attribute::ContentType, mime.into());
        let opts = PutOptions { attributes, ..Default::default() };
        self.store
            .put_opts(&path, image_bytes.to_vec().into(), opts)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(key.to_string())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError> {
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

    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        let path = Path::from(key);
        match self.store.delete(&path).await {
            Ok(()) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(DomainError::InfrastructureError(e.to_string())),
        }
    }
}

pub struct ImageCleanupHandler {
    image_storage: Arc<dyn ImageStorage>,
}

impl ImageCleanupHandler {
    pub fn new(image_storage: Arc<dyn ImageStorage>) -> Self {
        Self { image_storage }
    }
}

#[async_trait]
impl EventHandler for ImageCleanupHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let poster_path = match event {
            DomainEvent::MovieDeleted { poster_path, .. } => poster_path,
            _ => return Ok(()),
        };
        let Some(path) = poster_path else { return Ok(()) };
        if let Err(e) = self.image_storage.delete(path.value()).await {
            tracing::warn!("image cleanup failed for {}: {e}", path.value());
        }
        Ok(())
    }
}

pub fn create() -> anyhow::Result<Arc<dyn ImageStorage>> {
    Ok(Arc::new(ImageStorageAdapter::from_config(StorageConfig::from_env()?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;

    fn adapter() -> ImageStorageAdapter {
        ImageStorageAdapter::new(Arc::new(InMemory::new()))
    }

    #[tokio::test]
    async fn store_and_retrieve_round_trip() {
        let adapter = adapter();
        let bytes = b"fake-image-bytes";
        let path = adapter.store("posters/abc123", bytes).await.unwrap();
        assert_eq!(path, "posters/abc123");
        let retrieved = adapter.get("posters/abc123").await.unwrap();
        assert_eq!(retrieved, bytes);
    }

    #[tokio::test]
    async fn get_missing_returns_not_found() {
        let adapter = adapter();
        let result = adapter.get("nonexistent").await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let adapter = adapter();
        adapter.store("avatars/user1", b"img").await.unwrap();
        adapter.delete("avatars/user1").await.unwrap();
        let result = adapter.get("avatars/user1").await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_missing_returns_ok() {
        let adapter = adapter();
        assert!(adapter.delete("does-not-exist").await.is_ok());
    }

    #[tokio::test]
    async fn cleanup_handler_deletes_on_movie_deleted() {
        use domain::{events::DomainEvent, value_objects::{MovieId, PosterPath}};
        let inner = Arc::new(adapter());
        inner.store("some-uuid", b"img").await.unwrap();
        let path = PosterPath::new("some-uuid".to_string()).unwrap();
        let handler = ImageCleanupHandler::new(Arc::clone(&inner) as Arc<dyn ImageStorage>);
        handler
            .handle(&DomainEvent::MovieDeleted {
                movie_id: MovieId::from_uuid(uuid::Uuid::new_v4()),
                poster_path: Some(path.clone()),
            })
            .await
            .unwrap();
        assert!(matches!(inner.get("some-uuid").await, Err(DomainError::NotFound(_))));
    }
}
