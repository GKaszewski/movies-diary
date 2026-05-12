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
}

impl NatsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let url = std::env::var("NATS_URL")
            .map_err(|_| anyhow::anyhow!("NATS_URL is not set"))?;

        let mode = match std::env::var("NATS_MODE")
            .unwrap_or_else(|_| "jetstream".to_string())
            .as_str()
        {
            "core"      => NatsMode::Core,
            "jetstream" => NatsMode::JetStream,
            other       => anyhow::bail!("unknown NATS_MODE: {other}"),
        };

        let subject_prefix = std::env::var("NATS_SUBJECT_PREFIX")
            .unwrap_or_else(|_| "movies-diary.events".to_string());
        let stream_name = std::env::var("NATS_STREAM_NAME")
            .unwrap_or_else(|_| "MOVIES_DIARY_EVENTS".to_string());
        let consumer_name = std::env::var("NATS_CONSUMER_NAME")
            .unwrap_or_else(|_| "worker".to_string());

        Ok(Self { url, mode, subject_prefix, stream_name, consumer_name })
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
