#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NatsMode {
    Core,
    JetStream,
}

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub mode: NatsMode,
    pub subject_prefix: String,
    pub stream_name: String,
    pub consumer_name: String,
    pub max_deliver: i64,
}

impl NatsConfig {
    pub fn from_config(
        config: &::config::NatsConfig,
        events: &::config::EventsConfig,
    ) -> anyhow::Result<Self> {
        let url = config
            .url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NATS_URL is not set"))?;

        let mode = match config.mode.as_str() {
            "core" => NatsMode::Core,
            "jetstream" => NatsMode::JetStream,
            other => anyhow::bail!("unknown NATS_MODE: {other}"),
        };

        Ok(Self {
            url,
            mode,
            subject_prefix: config.subject_prefix.clone(),
            stream_name: config.stream_name.clone(),
            consumer_name: config.consumer_name.clone(),
            max_deliver: i64::from(events.queue_max_attempts),
        })
    }
}
