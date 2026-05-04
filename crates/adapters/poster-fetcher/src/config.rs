pub struct PosterFetcherConfig {
    pub timeout_seconds: u64,
}

impl PosterFetcherConfig {
    pub fn from_env() -> Self {
        let timeout_seconds = std::env::var("POSTER_FETCH_TIMEOUT_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Self { timeout_seconds }
    }
}
