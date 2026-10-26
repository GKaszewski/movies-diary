use serde::Deserialize;

use crate::lenient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ImageConversionConfig {
    #[serde(deserialize_with = "lenient::true_only")]
    pub enabled: bool,
    pub format: Option<String>,
}
