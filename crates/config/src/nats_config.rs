use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NatsConfig {
    pub url: Option<String>,
    pub mode: String,
    pub subject_prefix: String,
    pub stream_name: String,
    pub consumer_name: String,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: None,
            mode: "jetstream".into(),
            subject_prefix: "movies-diary.events".into(),
            stream_name: "MOVIES_DIARY_EVENTS".into(),
            consumer_name: "worker".into(),
        }
    }
}
