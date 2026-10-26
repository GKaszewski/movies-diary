use object_storage::StorageConfig;

fn local_section(dir: &std::path::Path) -> config::StorageConfig {
    config::StorageConfig {
        backend: "local".into(),
        path: dir.to_str().unwrap().into(),
        ..Default::default()
    }
}

#[test]
fn local_store_creates_dir_and_succeeds() {
    let dir = std::env::temp_dir().join(format!("image_test_{}", uuid::Uuid::new_v4()));
    let result = StorageConfig::from_config(&local_section(&dir));
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    assert!(dir.exists(), "directory should have been created");
}

#[test]
fn local_store_succeeds_if_dir_already_exists() {
    let dir = std::env::temp_dir().join(format!("image_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let result = StorageConfig::from_config(&local_section(&dir));
    assert!(result.is_ok());
}

#[test]
fn s3_backend_requires_endpoint() {
    let section = config::StorageConfig {
        backend: "s3".into(),
        ..Default::default()
    };
    let err = StorageConfig::from_config(&section).err().unwrap();
    assert!(err.to_string().contains("MINIO_ENDPOINT required"));
}

#[test]
fn unknown_backend_is_error() {
    let section = config::StorageConfig {
        backend: "ftp".into(),
        ..Default::default()
    };
    assert!(StorageConfig::from_config(&section).is_err());
}
