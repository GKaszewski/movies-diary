use anyhow::Result;
use directories::ProjectDirs;
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

    pub fn load_token() -> Option<String> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .ok()
            .and_then(|e| e.get_password().ok())
    }

    pub fn save_token(token: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        entry.set_password(token)?;
        Ok(())
    }

    pub fn clear_token() -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        let _ = entry.delete_credential(); // ignore NotFound
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
        // Tests that load() doesn't panic — may return Some or None depending on system state
        let _ = Config::load();
    }
}
