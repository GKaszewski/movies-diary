use domain::errors::DomainError;
use domain::ports::ObjectStorage;
use domain::value_objects::WrapUpId;
use std::sync::Arc;

pub struct WrapUpStorage {
    inner: Arc<dyn ObjectStorage>,
}

impl WrapUpStorage {
    pub fn new(storage: Arc<dyn ObjectStorage>) -> Self {
        Self { inner: storage }
    }

    pub async fn store_video(&self, id: &WrapUpId, bytes: &[u8]) -> Result<(), DomainError> {
        let key = format!("wrapups/{}/video.mp4", id.value());
        self.inner.store(&key, bytes).await?;
        Ok(())
    }

    pub async fn delete_video(&self, id: &WrapUpId) -> Result<(), DomainError> {
        let key = format!("wrapups/{}/video.mp4", id.value());
        self.inner.delete(&key).await
    }

    pub fn cast_image_key(profile_path: &str) -> String {
        format!("cast{profile_path}")
    }

    pub async fn resolve_cast_images(&self, profile_paths: &[String]) -> Vec<(String, Vec<u8>)> {
        let mut images = Vec::new();
        for path in profile_paths.iter().take(20) {
            let key = Self::cast_image_key(path);
            match self.inner.get(&key).await {
                Ok(bytes) => images.push((key, bytes)),
                Err(e) => tracing::debug!("cast fetch skipped for {key}: {e}"),
            }
        }
        tracing::info!(
            "resolved {}/{} cast images",
            images.len(),
            profile_paths.len()
        );
        images
    }

    pub async fn resolve_poster_images(&self, paths: &[String]) -> Vec<(String, Vec<u8>)> {
        let mut images = Vec::new();
        for path in paths.iter().take(20) {
            match self.inner.get(path).await {
                Ok(bytes) => images.push((path.clone(), bytes)),
                Err(e) => tracing::debug!("poster fetch skipped for {path}: {e}"),
            }
        }
        tracing::info!("resolved {}/{} poster images", images.len(), paths.len());
        images
    }
}
