use nats::{NatsConfig, NatsMode};

fn section(url: Option<&str>, mode: &str) -> config::NatsConfig {
    config::NatsConfig {
        url: url.map(str::to_owned),
        mode: mode.into(),
        ..Default::default()
    }
}

#[test]
fn errors_without_nats_url() {
    assert!(
        NatsConfig::from_config(
            &config::NatsConfig::default(),
            &config::EventsConfig::default()
        )
        .is_err()
    );
}

#[test]
fn defaults_with_only_url() {
    let cfg = NatsConfig::from_config(
        &config::NatsConfig {
            url: Some("nats://localhost:4222".into()),
            ..Default::default()
        },
        &config::EventsConfig::default(),
    )
    .unwrap();
    assert_eq!(cfg.url, "nats://localhost:4222");
    assert_eq!(cfg.mode, NatsMode::JetStream);
    assert_eq!(cfg.subject_prefix, "movies-diary.events");
    assert_eq!(cfg.stream_name, "MOVIES_DIARY_EVENTS");
    assert_eq!(cfg.consumer_name, "worker");
}

#[test]
fn core_mode_parsed() {
    let cfg = NatsConfig::from_config(
        &section(Some("nats://test:4222"), "core"),
        &config::EventsConfig::default(),
    )
    .unwrap();
    assert_eq!(cfg.url, "nats://test:4222");
    assert_eq!(cfg.mode, NatsMode::Core);
}

#[test]
fn invalid_mode_errors() {
    assert!(
        NatsConfig::from_config(
            &section(Some("nats://test:4222"), "kafka"),
            &config::EventsConfig::default()
        )
        .is_err()
    );
}

#[test]
fn max_deliver_follows_the_queue_attempt_limit() {
    let events = config::EventsConfig {
        queue_max_attempts: 9,
        ..Default::default()
    };
    let cfg =
        NatsConfig::from_config(&section(Some("nats://test:4222"), "jetstream"), &events).unwrap();
    assert_eq!(cfg.max_deliver, 9);
}
