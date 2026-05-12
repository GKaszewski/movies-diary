use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, ImageRefCommand, ImageStorage},
};

use crate::Format;

pub struct ImageConversionHandler {
    storage: Arc<dyn ImageStorage>,
    image_ref: Arc<dyn ImageRefCommand>,
    format: Format,
}

impl ImageConversionHandler {
    pub fn new(
        storage: Arc<dyn ImageStorage>,
        image_ref: Arc<dyn ImageRefCommand>,
        format: Format,
    ) -> Self {
        Self { storage, image_ref, format }
    }
}

#[async_trait]
impl EventHandler for ImageConversionHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let key = match event {
            DomainEvent::ImageStored { key } => key.clone(),
            _ => return Ok(()),
        };

        if key.ends_with(".avif") || key.ends_with(".webp") {
            return Ok(());
        }

        let bytes = self.storage.get(&key).await?;
        let format = self.format;

        let converted = tokio::task::spawn_blocking(move || convert(bytes, format))
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .map_err(|e| DomainError::InfrastructureError(e))?;

        let ext = format.extension();
        let new_key = format!("{key}{ext}");
        self.storage.store(&new_key, &converted).await?;

        if let Err(e) = self.image_ref.swap(&key, &new_key).await {
            tracing::error!("swap failed for {key} → {new_key}: {e}");
            return Err(e);
        }

        if let Err(e) = self.storage.delete(&key).await {
            tracing::warn!("failed to delete old image key {key}: {e}");
        }

        tracing::info!("converted {key} → {new_key}");
        Ok(())
    }
}

fn convert(bytes: Vec<u8>, format: Format) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;

    match format {
        Format::Avif => {
            let rgba = img.to_rgba8();
            let width = rgba.width() as usize;
            let height = rgba.height() as usize;
            let pixels: Vec<ravif::RGBA8> = rgba
                .pixels()
                .map(|p| ravif::RGBA8 { r: p.0[0], g: p.0[1], b: p.0[2], a: p.0[3] })
                .collect();
            let result = ravif::Encoder::new()
                .with_quality(80.0)
                .with_speed(6)
                .encode_rgba(ravif::Img::new(&pixels, width, height))
                .map_err(|e| e.to_string())?;
            Ok(result.avif_file.to_vec())
        }
        Format::Webp => {
            let rgba = img.to_rgba8();
            let (width, height) = (rgba.width(), rgba.height());
            let encoder = webp::Encoder::from_rgba(rgba.as_raw(), width, height);
            Ok(encoder.encode(80.0).to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use object_store::memory::InMemory;
    use image_storage::ImageStorageAdapter;

    struct MockImageRef {
        swaps: Mutex<Vec<(String, String)>>,
    }

    impl MockImageRef {
        fn new() -> Arc<Self> {
            Arc::new(Self { swaps: Mutex::new(vec![]) })
        }

        fn swaps(&self) -> Vec<(String, String)> {
            self.swaps.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl ImageRefCommand for MockImageRef {
        async fn swap(&self, old: &str, new: &str) -> Result<(), DomainError> {
            self.swaps.lock().unwrap().push((old.into(), new.into()));
            Ok(())
        }
    }

    fn in_memory_storage() -> Arc<ImageStorageAdapter> {
        Arc::new(ImageStorageAdapter::new(Arc::new(InMemory::new())))
    }

    fn tiny_jpeg() -> Vec<u8> {
        use image::{DynamicImage, ImageBuffer, Rgb};
        let img = DynamicImage::ImageRgb8(
            ImageBuffer::from_pixel(4, 4, Rgb([200u8, 100, 50])),
        );
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[tokio::test]
    async fn ignores_non_image_stored_events() {
        let storage = in_memory_storage();
        let image_ref = MockImageRef::new();
        let handler = ImageConversionHandler::new(
            Arc::clone(&storage) as Arc<dyn ImageStorage>,
            Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
            Format::Avif,
        );

        handler.handle(&DomainEvent::UserUpdated {
            user_id: domain::value_objects::UserId::from_uuid(uuid::Uuid::new_v4()),
        }).await.unwrap();

        assert!(image_ref.swaps().is_empty());
    }

    #[tokio::test]
    async fn skips_already_converted_avif_key() {
        let storage = in_memory_storage();
        storage.store("avatars/u1.avif", &tiny_jpeg()).await.unwrap();
        let image_ref = MockImageRef::new();
        let handler = ImageConversionHandler::new(
            Arc::clone(&storage) as Arc<dyn ImageStorage>,
            Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
            Format::Avif,
        );

        handler.handle(&DomainEvent::ImageStored { key: "avatars/u1.avif".into() }).await.unwrap();

        assert!(image_ref.swaps().is_empty());
    }

    #[tokio::test]
    async fn skips_already_converted_webp_key() {
        let storage = in_memory_storage();
        storage.store("posters/m1.webp", &tiny_jpeg()).await.unwrap();
        let image_ref = MockImageRef::new();
        let handler = ImageConversionHandler::new(
            Arc::clone(&storage) as Arc<dyn ImageStorage>,
            Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
            Format::Webp,
        );

        handler.handle(&DomainEvent::ImageStored { key: "posters/m1.webp".into() }).await.unwrap();

        assert!(image_ref.swaps().is_empty());
    }

    #[tokio::test]
    async fn converts_jpeg_to_avif_and_swaps_key() {
        let storage = in_memory_storage();
        storage.store("avatars/u1", &tiny_jpeg()).await.unwrap();
        let image_ref = MockImageRef::new();
        let handler = ImageConversionHandler::new(
            Arc::clone(&storage) as Arc<dyn ImageStorage>,
            Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
            Format::Avif,
        );

        handler.handle(&DomainEvent::ImageStored { key: "avatars/u1".into() }).await.unwrap();

        assert_eq!(image_ref.swaps(), vec![("avatars/u1".into(), "avatars/u1.avif".into())]);
        assert!(storage.get("avatars/u1.avif").await.is_ok());
        assert!(storage.get("avatars/u1").await.is_err());
    }

    #[tokio::test]
    async fn converts_jpeg_to_webp_and_swaps_key() {
        let storage = in_memory_storage();
        storage.store("avatars/u1", &tiny_jpeg()).await.unwrap();
        let image_ref = MockImageRef::new();
        let handler = ImageConversionHandler::new(
            Arc::clone(&storage) as Arc<dyn ImageStorage>,
            Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
            Format::Webp,
        );

        handler.handle(&DomainEvent::ImageStored { key: "avatars/u1".into() }).await.unwrap();

        assert_eq!(image_ref.swaps(), vec![("avatars/u1".into(), "avatars/u1.webp".into())]);
        assert!(storage.get("avatars/u1.webp").await.is_ok());
        assert!(storage.get("avatars/u1").await.is_err());
    }
}
