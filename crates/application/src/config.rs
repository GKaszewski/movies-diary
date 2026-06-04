#[derive(Clone)]
pub struct AppConfig {
    pub allow_registration: bool,
    pub base_url: String,
    pub rate_limit: u64,
    pub wrapup: WrapUpConfig,
}

#[derive(Clone)]
pub struct WrapUpConfig {
    pub font_path: Option<String>,
    pub logo_path: Option<String>,
    pub bg_dir: Option<String>,
    pub ffmpeg_path: String,
    pub max_concurrent_renders: usize,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let allow_registration = std::env::var("ALLOW_REGISTRATION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let rate_limit = std::env::var("RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        Self {
            allow_registration,
            base_url,
            rate_limit,
            wrapup: WrapUpConfig::from_env(),
        }
    }
}

impl WrapUpConfig {
    pub fn from_env() -> Self {
        Self {
            font_path: std::env::var("WRAPUP_FONT_PATH").ok(),
            logo_path: std::env::var("WRAPUP_LOGO_PATH").ok(),
            bg_dir: std::env::var("WRAPUP_BG_DIR").ok(),
            ffmpeg_path: std::env::var("FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
            max_concurrent_renders: std::env::var("WRAPUP_MAX_CONCURRENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
        }
    }
}
