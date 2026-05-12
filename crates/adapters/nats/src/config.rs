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
        Self::from_vars(
            std::env::var("NATS_URL").ok().as_deref(),
            std::env::var("NATS_MODE").ok().as_deref(),
            std::env::var("NATS_SUBJECT_PREFIX").ok().as_deref(),
            std::env::var("NATS_STREAM_NAME").ok().as_deref(),
            std::env::var("NATS_CONSUMER_NAME").ok().as_deref(),
        )
    }

    pub(crate) fn from_vars(
        url: Option<&str>,
        mode: Option<&str>,
        subject_prefix: Option<&str>,
        stream_name: Option<&str>,
        consumer_name: Option<&str>,
    ) -> anyhow::Result<Self> {
        let url = url.ok_or_else(|| anyhow::anyhow!("NATS_URL is not set"))?;

        let mode = match mode.unwrap_or("jetstream") {
            "core"      => NatsMode::Core,
            "jetstream" => NatsMode::JetStream,
            other       => anyhow::bail!("unknown NATS_MODE: {other}"),
        };

        let subject_prefix = subject_prefix.unwrap_or("movies-diary.events").to_string();
        let stream_name = stream_name.unwrap_or("MOVIES_DIARY_EVENTS").to_string();
        let consumer_name = consumer_name.unwrap_or("worker").to_string();

        Ok(Self { url: url.to_string(), mode, subject_prefix, stream_name, consumer_name })
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
