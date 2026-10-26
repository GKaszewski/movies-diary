#[test]
fn tmdb_client_builds_without_bootstrap() {
    let _ = metadata::MetadataClientImpl::new_tmdb("key".into());
}
