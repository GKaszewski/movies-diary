#[derive(Clone)]
pub struct AppConfig {
    pub allow_registration: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let allow_registration = std::env::var("ALLOW_REGISTRATION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        Self { allow_registration }
    }
}
