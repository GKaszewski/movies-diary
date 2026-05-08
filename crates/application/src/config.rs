#[derive(Clone)]
pub struct AppConfig {
    pub allow_registration: bool,
    pub base_url: String,
    pub rate_limit: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let allow_registration = std::env::var("ALLOW_REGISTRATION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let base_url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let rate_limit = std::env::var("RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);
        Self { allow_registration, base_url, rate_limit }
    }
}
