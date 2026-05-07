use anyhow::Result;
use directories::ProjectDirs;
use keyring_core::Entry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
}

const KEYRING_SERVICE: &str = "movie-tui";
const KEYRING_USER: &str = "jwt-token";

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "movies", "movie-tui")
        .map(|dirs| dirs.config_dir().join("config.json"))
}

impl Config {
    pub fn load() -> Option<Config> {
        let path = config_path()?;
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    #[allow(unreachable_code)]
    pub fn init_keyring() -> Result<()> {
        #[cfg(feature = "macos")]
        {
            use apple_native_keyring_store::keychain::Store;
            keyring_core::set_default_store(Store::new()?);
            return Ok(());
        }

        #[cfg(feature = "linux-zbus")]
        {
            keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new()?);
            return Ok(());
        }

        #[cfg(feature = "windows")]
        {
            keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
            return Ok(());
        }

        anyhow::bail!(
            "no keyring backend compiled in — build with --features macos|linux-zbus|windows"
        )
    }

    pub fn load_token() -> Option<String> {
        Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .ok()
            .and_then(|e| e.get_password().ok())
    }

    pub fn save_token(token: &str) -> Result<()> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        entry.set_password(token)?;
        Ok(())
    }

    pub fn clear_token() -> Result<()> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        let _ = entry.delete_credential();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let config = Config { api_url: "http://localhost:3000".into() };
        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.api_url, "http://localhost:3000");
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let _ = Config::load();
    }
}
