pub struct PosterFetcherConfig {
    pub timeout_seconds: u64,
}

impl PosterFetcherConfig {
    pub fn from_config(config: &::config::MetadataConfig) -> Self {
        Self {
            timeout_seconds: config.poster_fetch_timeout_seconds,
        }
    }
}
