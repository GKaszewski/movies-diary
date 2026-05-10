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
mod tests {
    use super::*;

    #[test]
    fn errors_without_nats_url() {
        unsafe { std::env::remove_var("NATS_URL"); }
        assert!(NatsConfig::from_env().is_err());
    }

    #[test]
    fn defaults_with_only_url() {
        unsafe {
            std::env::set_var("NATS_URL", "nats://localhost:4222");
            std::env::remove_var("NATS_MODE");
            std::env::remove_var("NATS_SUBJECT_PREFIX");
            std::env::remove_var("NATS_STREAM_NAME");
            std::env::remove_var("NATS_CONSUMER_NAME");
        }

        let cfg = NatsConfig::from_env().unwrap();
        assert_eq!(cfg.url, "nats://localhost:4222");
        assert_eq!(cfg.mode, NatsMode::JetStream);
        assert_eq!(cfg.subject_prefix, "movies-diary.events");
        assert_eq!(cfg.stream_name, "MOVIES_DIARY_EVENTS");
        assert_eq!(cfg.consumer_name, "worker");

        unsafe { std::env::remove_var("NATS_URL"); }
    }

    #[test]
    fn core_mode_parsed() {
        unsafe {
            std::env::set_var("NATS_URL", "nats://test:4222");
            std::env::set_var("NATS_MODE", "core");
        }

        let cfg = NatsConfig::from_env().unwrap();
        assert_eq!(cfg.mode, NatsMode::Core);

        unsafe {
            std::env::remove_var("NATS_URL");
            std::env::remove_var("NATS_MODE");
        }
    }

    #[test]
    fn invalid_mode_errors() {
        unsafe {
            std::env::set_var("NATS_URL", "nats://test:4222");
            std::env::set_var("NATS_MODE", "kafka");
        }

        assert!(NatsConfig::from_env().is_err());

        unsafe {
            std::env::remove_var("NATS_URL");
            std::env::remove_var("NATS_MODE");
        }
    }
}
