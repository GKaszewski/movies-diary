use super::*;

#[test]
fn errors_without_nats_url() {
    assert!(NatsConfig::from_vars(None, None, None, None, None).is_err());
}

#[test]
fn defaults_with_only_url() {
    let cfg = NatsConfig::from_vars(Some("nats://localhost:4222"), None, None, None, None).unwrap();
    assert_eq!(cfg.url, "nats://localhost:4222");
    assert_eq!(cfg.mode, NatsMode::JetStream);
    assert_eq!(cfg.subject_prefix, "movies-diary.events");
    assert_eq!(cfg.stream_name, "MOVIES_DIARY_EVENTS");
    assert_eq!(cfg.consumer_name, "worker");
}

#[test]
fn core_mode_parsed() {
    let cfg =
        NatsConfig::from_vars(Some("nats://test:4222"), Some("core"), None, None, None).unwrap();
    assert_eq!(cfg.mode, NatsMode::Core);
}

#[test]
fn invalid_mode_errors() {
    assert!(
        NatsConfig::from_vars(Some("nats://test:4222"), Some("kafka"), None, None, None).is_err()
    );
}
