use tui::config::Config;

#[test]
fn config_survives_json_round_trip() {
    let config = Config {
        api_url: "http://localhost:3000".into(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let decoded: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.api_url, "http://localhost:3000");
}

#[test]
fn load_without_config_file_does_not_panic() {
    let _ = Config::load();
}
