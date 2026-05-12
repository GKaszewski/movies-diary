use super::*;

#[test]
fn local_store_creates_dir_and_succeeds() {
    let dir = std::env::temp_dir().join(format!("image_test_{}", uuid::Uuid::new_v4()));
    let result = build_local_store(dir.to_str().unwrap());
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    assert!(dir.exists(), "directory should have been created");
}

#[test]
fn local_store_succeeds_if_dir_already_exists() {
    let dir = std::env::temp_dir().join(format!("image_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let result = build_local_store(dir.to_str().unwrap());
    assert!(result.is_ok());
}
