use tmdb_enrichment::TmdbEnrichmentClient;

#[test]
fn tmdb_enrichment_builds_without_bootstrap() {
    let cfg = config::MetadataConfig {
        tmdb_api_key: Some("key".into()),
        ..Default::default()
    };
    TmdbEnrichmentClient::from_config(&cfg).expect("client builds");
}
