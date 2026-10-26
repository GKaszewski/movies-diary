use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use domain::{errors::DomainError, ports::ObjectStorage};

use application::images::{deps::GetImageDeps, get_image};

#[derive(Default)]
struct Storage {
    objects: Mutex<HashMap<String, Vec<u8>>>,
}

#[async_trait]
impl ObjectStorage for Storage {
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError> {
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_string(), image_bytes.to_vec());
        Ok(key.to_string())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError> {
        self.objects
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| DomainError::NotFound(key.to_string()))
    }

    async fn get_stream(
        &self,
        _key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        unimplemented!()
    }

    async fn delete(&self, _key: &str) -> Result<(), DomainError> {
        unimplemented!()
    }

    async fn exists(&self, key: &str) -> Result<bool, DomainError> {
        Ok(self.objects.lock().unwrap().contains_key(key))
    }
}

#[tokio::test]
async fn returns_the_stored_bytes() {
    let storage = Arc::new(Storage::default());
    storage.store("posters/1.webp", b"image").await.unwrap();
    let deps = GetImageDeps {
        object_storage: storage,
    };

    let bytes = get_image::execute(&deps, "posters/1.webp").await.unwrap();

    assert_eq!(bytes, b"image");
}

#[tokio::test]
async fn a_missing_key_is_not_found() {
    let deps = GetImageDeps {
        object_storage: Arc::new(Storage::default()),
    };

    let err = get_image::execute(&deps, "posters/missing.webp")
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::NotFound(_)));
}
