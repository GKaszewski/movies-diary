mod config;
pub use config::StorageConfig;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::PosterStorage,
    value_objects::{MovieId, PosterPath},
};
use object_store::{path::Path, ObjectStore};
use std::sync::Arc;

pub struct PosterStorageAdapter {
    store: Arc<dyn ObjectStore>,
}

impl PosterStorageAdapter {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    pub fn from_config(config: StorageConfig) -> anyhow::Result<Self> {
        Ok(Self::new(config.build_store()?))
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
        self.store
            .put(&path, image_bytes.to_vec().into())
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        PosterPath::new(path.to_string())
    }

    async fn get_poster(&self, poster_path: &PosterPath) -> Result<Vec<u8>, DomainError> {
        let path = Path::from(poster_path.value().to_string());
        let result = self.store.get(&path).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => DomainError::NotFound("Poster not found".into()),
            _ => DomainError::InfrastructureError(e.to_string()),
        })?;
        result
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
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
}
