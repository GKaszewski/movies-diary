#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("configuration file {0} was named by CONFIG_FILE but does not exist")]
    NamedFileMissing(String),
    #[error("{0} is required but not set")]
    Missing(&'static str),
    #[error("invalid value for {variable}: {reason}")]
    Invalid { variable: String, reason: String },
    #[error("could not read configuration: {0}")]
    Unreadable(#[from] Box<figment::Error>),
}
