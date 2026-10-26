use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WrapUpConfig {
    pub font_path: Option<String>,
    pub logo_path: Option<String>,
    pub bg_dir: Option<String>,
}
