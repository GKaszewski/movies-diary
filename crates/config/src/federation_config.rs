use serde::Deserialize;

use crate::lenient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FederationConfig {
    #[serde(deserialize_with = "lenient::true_or_one")]
    pub debug: bool,
}
