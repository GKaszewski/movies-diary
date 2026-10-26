#[derive(Clone)]
pub struct AppSettings {
    pub allow_registration: bool,
    pub base_url: String,
    pub rate_limit: u64,
    pub refresh_ttl_seconds: u64,
    pub secure_cookies: bool,
    pub cors_origins: String,
    pub wrapup: WrapUpSettings,
}

#[derive(Clone)]
pub struct WrapUpSettings {
    pub font_path: Option<String>,
    pub logo_path: Option<String>,
    pub bg_dir: Option<String>,
}

impl From<&::config::AppConfig> for AppSettings {
    fn from(config: &::config::AppConfig) -> Self {
        Self {
            allow_registration: config.server.allow_registration,
            base_url: config.server.base_url.clone(),
            rate_limit: config.server.rate_limit,
            refresh_ttl_seconds: config.auth.refresh_ttl_seconds,
            secure_cookies: config.server.secure_cookies,
            cors_origins: config.server.cors_origins.clone(),
            wrapup: WrapUpSettings::from(&config.wrapup),
        }
    }
}

impl From<&::config::WrapUpConfig> for WrapUpSettings {
    fn from(config: &::config::WrapUpConfig) -> Self {
        Self {
            font_path: config.font_path.clone(),
            logo_path: config.logo_path.clone(),
            bg_dir: config.bg_dir.clone(),
        }
    }
}
