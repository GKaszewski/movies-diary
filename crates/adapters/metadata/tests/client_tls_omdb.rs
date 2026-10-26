#[test]
fn omdb_client_builds_without_bootstrap() {
    let _ = metadata::MetadataClientImpl::new_omdb("key".into());
}
