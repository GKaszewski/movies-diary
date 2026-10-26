use serde::Deserialize;

use crate::lenient;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EventsConfig {
    pub bus_backend: String,
    #[serde(deserialize_with = "lenient::number")]
    pub channel_buffer: usize,
    #[serde(deserialize_with = "lenient::number")]
    pub queue_poll_interval_ms: u64,
    #[serde(deserialize_with = "lenient::number")]
    pub queue_batch_size: i64,
    #[serde(deserialize_with = "lenient::number")]
    pub queue_max_attempts: i32,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            bus_backend: "db".into(),
            channel_buffer: 128,
            queue_poll_interval_ms: 500,
            queue_batch_size: 10,
            queue_max_attempts: 5,
        }
    }
}
