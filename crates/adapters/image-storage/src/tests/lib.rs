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
    use domain::{
        events::DomainEvent,
        value_objects::{MovieId, PosterPath},
    };
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
    assert!(matches!(
        inner.get("some-uuid").await,
        Err(DomainError::NotFound(_))
    ));
}
