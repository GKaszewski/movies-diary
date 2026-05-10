mod config;
pub use config::StorageConfig;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, PosterStorage},
    value_objects::{MovieId, PosterPath},
};
use object_store::{Attribute, Attributes, ObjectStore, PutOptions, path::Path};
use std::sync::Arc;

fn detect_mime(bytes: &[u8]) -> &'static str {
    infer::get(bytes)
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream")
}

pub struct PosterStorageAdapter {
    store: Arc<dyn ObjectStore>,
}

impl PosterStorageAdapter {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    pub fn from_config(config: StorageConfig) -> Self {
        Self::new(config.build_store())
    }
}

#[async_trait]
impl PosterStorage for PosterStorageAdapter {
    async fn store_poster(
        &self,
        movie_id: &MovieId,
        image_bytes: &[u8],
    ) -> Result<PosterPath, DomainError> {
        let path = Path::from(movie_id.value().to_string());
        let mime = detect_mime(image_bytes);
        let mut attributes = Attributes::new();
        attributes.insert(Attribute::ContentType, mime.into());
        let opts = PutOptions {
            attributes,
            ..Default::default()
        };
        self.store
            .put_opts(&path, image_bytes.to_vec().into(), opts)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        PosterPath::new(path.to_string())
    }

    async fn delete_poster(&self, path: &PosterPath) -> Result<(), DomainError> {
        let p = Path::from(path.value().to_string());
        match self.store.delete(&p).await {
            Ok(()) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(DomainError::InfrastructureError(e.to_string())),
        }
    }

    async fn get_poster(&self, poster_path: &PosterPath) -> Result<Vec<u8>, DomainError> {
        let path = Path::from(poster_path.value().to_string());
        let result = self.store.get(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => {
                DomainError::NotFound("Poster not found".into())
            }
            _ => DomainError::InfrastructureError(e.to_string()),
        })?;
        result
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

pub struct PosterCleanupHandler {
    poster_storage: Arc<dyn PosterStorage>,
}

impl PosterCleanupHandler {
    pub fn new(poster_storage: Arc<dyn PosterStorage>) -> Self {
        Self { poster_storage }
    }
}

#[async_trait]
impl EventHandler for PosterCleanupHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let poster_path = match event {
            DomainEvent::MovieDeleted { poster_path, .. } => poster_path,
            _ => return Ok(()),
        };
        let Some(path) = poster_path else { return Ok(()) };
        if let Err(e) = self.poster_storage.delete_poster(path).await {
            tracing::warn!("poster cleanup failed for {}: {e}", path.value());
        }
        Ok(())
    }
}

pub fn create() -> anyhow::Result<std::sync::Arc<dyn domain::ports::PosterStorage>> {
    Ok(std::sync::Arc::new(PosterStorageAdapter::from_config(StorageConfig::from_env()?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;
    use uuid::Uuid;

    fn adapter() -> PosterStorageAdapter {
        PosterStorageAdapter::new(Arc::new(InMemory::new()))
    }

    #[tokio::test]
    async fn store_and_retrieve_round_trip() {
        let adapter = adapter();
        let movie_id = MovieId::from_uuid(Uuid::new_v4());
        let bytes = b"fake-image-bytes";

        let path = adapter.store_poster(&movie_id, bytes).await.unwrap();
        let retrieved = adapter.get_poster(&path).await.unwrap();

        assert_eq!(retrieved, bytes);
    }

    #[tokio::test]
    async fn get_missing_returns_not_found() {
        let adapter = adapter();
        let path = PosterPath::new("nonexistent".into()).unwrap();
        let result = adapter.get_poster(&path).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_poster_removes_file() {
        let adapter = adapter();
        let movie_id = MovieId::from_uuid(Uuid::new_v4());
        let path = adapter.store_poster(&movie_id, b"img").await.unwrap();

        adapter.delete_poster(&path).await.unwrap();

        let result = adapter.get_poster(&path).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_poster_missing_file_returns_ok() {
        let adapter = adapter();
        let path = PosterPath::new("does-not-exist".into()).unwrap();
        assert!(adapter.delete_poster(&path).await.is_ok());
    }

    #[tokio::test]
    async fn cleanup_handler_deletes_poster_on_movie_deleted() {
        use domain::{events::DomainEvent, ports::EventHandler};

        let inner = Arc::new(adapter());
        let path = inner
            .store_poster(&MovieId::from_uuid(Uuid::new_v4()), b"img")
            .await
            .unwrap();
        let movie_id = MovieId::from_uuid(Uuid::new_v4());

        let handler = PosterCleanupHandler::new(Arc::clone(&inner) as Arc<dyn PosterStorage>);
        handler
            .handle(&DomainEvent::MovieDeleted { movie_id, poster_path: Some(path.clone()) })
            .await
            .unwrap();

        assert!(matches!(inner.get_poster(&path).await, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn cleanup_handler_ignores_none_poster_path() {
        use domain::{events::DomainEvent, ports::EventHandler};

        let inner = Arc::new(adapter());
        let handler = PosterCleanupHandler::new(Arc::clone(&inner) as Arc<dyn PosterStorage>);
        let event = DomainEvent::MovieDeleted {
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            poster_path: None,
        };
        handler.handle(&event).await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_handler_ignores_other_events() {
        use domain::{events::DomainEvent, ports::EventHandler, value_objects::ExternalMetadataId};

        let inner = Arc::new(adapter());
        let handler = PosterCleanupHandler::new(Arc::clone(&inner) as Arc<dyn PosterStorage>);
        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            external_metadata_id: ExternalMetadataId::new("tt1234567".to_string()).unwrap(),
        };
        handler.handle(&event).await.unwrap();
    }
}
