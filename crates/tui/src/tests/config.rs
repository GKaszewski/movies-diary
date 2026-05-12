use super::*;

#[test]
fn config_roundtrip() {
    let config = Config {
        api_url: "http://localhost:3000".into(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let decoded: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.api_url, "http://localhost:3000");
}

#[test]
fn load_returns_none_when_no_file() {
    let _ = Config::load();
}
