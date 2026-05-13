use super::*;
use image_storage::ImageStorageAdapter;
use object_store::memory::InMemory;
use std::sync::Mutex;

struct MockImageRef {
    swaps: Mutex<Vec<(String, String)>>,
}

impl MockImageRef {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            swaps: Mutex::new(vec![]),
        })
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
    let img = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(4, 4, Rgb([200u8, 100, 50])));
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

    handler
        .handle(&DomainEvent::UserUpdated {
            user_id: domain::value_objects::UserId::from_uuid(uuid::Uuid::new_v4()),
        })
        .await
        .unwrap();

    assert!(image_ref.swaps().is_empty());
}

#[tokio::test]
async fn skips_already_converted_avif_key() {
    let storage = in_memory_storage();
    storage
        .store("avatars/u1.avif", &tiny_jpeg())
        .await
        .unwrap();
    let image_ref = MockImageRef::new();
    let handler = ImageConversionHandler::new(
        Arc::clone(&storage) as Arc<dyn ImageStorage>,
        Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
        Format::Avif,
    );

    handler
        .handle(&DomainEvent::ImageStored {
            key: "avatars/u1.avif".into(),
        })
        .await
        .unwrap();

    assert!(image_ref.swaps().is_empty());
}

#[tokio::test]
async fn skips_already_converted_webp_key() {
    let storage = in_memory_storage();
    storage
        .store("posters/m1.webp", &tiny_jpeg())
        .await
        .unwrap();
    let image_ref = MockImageRef::new();
    let handler = ImageConversionHandler::new(
        Arc::clone(&storage) as Arc<dyn ImageStorage>,
        Arc::clone(&image_ref) as Arc<dyn ImageRefCommand>,
        Format::Webp,
    );

    handler
        .handle(&DomainEvent::ImageStored {
            key: "posters/m1.webp".into(),
        })
        .await
        .unwrap();

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

    handler
        .handle(&DomainEvent::ImageStored {
            key: "avatars/u1".into(),
        })
        .await
        .unwrap();

    assert_eq!(
        image_ref.swaps(),
        vec![("avatars/u1".into(), "avatars/u1.avif".into())]
    );
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

    handler
        .handle(&DomainEvent::ImageStored {
            key: "avatars/u1".into(),
        })
        .await
        .unwrap();

    assert_eq!(
        image_ref.swaps(),
        vec![("avatars/u1".into(), "avatars/u1.webp".into())]
    );
    assert!(storage.get("avatars/u1.webp").await.is_ok());
    assert!(storage.get("avatars/u1").await.is_err());
}
