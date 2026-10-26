use std::path::Path;

use figment::providers::{Format, Toml};
use figment::value::{Dict, Map, Value};
use figment::{Figment, Metadata, Profile, Provider};

use crate::AppConfig;
use crate::errors::ConfigError;

const DEFAULT_CONFIG_FILE: &str = "config.toml";
const CONFIG_FILE_VARIABLE: &str = "CONFIG_FILE";

const LEGACY_ENV: &[(&str, &str)] = &[
    ("ALLOW_REGISTRATION", "server.allow_registration"),
    ("BASE_URL", "server.base_url"),
    ("CORS_ORIGINS", "server.cors_origins"),
    ("DATABASE_URL", "database.url"),
    ("EVENT_BUS_BACKEND", "events.bus_backend"),
    ("EVENT_CHANNEL_BUFFER", "events.channel_buffer"),
    ("EVENT_QUEUE_BATCH_SIZE", "events.queue_batch_size"),
    ("EVENT_QUEUE_MAX_ATTEMPTS", "events.queue_max_attempts"),
    (
        "EVENT_QUEUE_POLL_INTERVAL_MS",
        "events.queue_poll_interval_ms",
    ),
    ("FEDERATION_DEBUG", "federation.debug"),
    ("HOST", "server.host"),
    ("PORT", "server.port"),
    ("IMAGE_CONVERSION_ENABLED", "image_conversion.enabled"),
    ("IMAGE_CONVERSION_FORMAT", "image_conversion.format"),
    ("IMAGE_STORAGE_BACKEND", "storage.backend"),
    ("IMAGE_STORAGE_PATH", "storage.path"),
    ("JWT_SECRET", "auth.jwt_secret"),
    ("JWT_TTL_SECONDS", "auth.jwt_ttl_seconds"),
    ("MINIO_ACCESS_KEY_ID", "storage.access_key_id"),
    ("MINIO_BUCKET", "storage.bucket"),
    ("MINIO_ENDPOINT", "storage.endpoint"),
    ("MINIO_REGION", "storage.region"),
    ("MINIO_SECRET_ACCESS_KEY", "storage.secret_access_key"),
    ("NATS_CONSUMER_NAME", "nats.consumer_name"),
    ("NATS_MODE", "nats.mode"),
    ("NATS_STREAM_NAME", "nats.stream_name"),
    ("NATS_SUBJECT_PREFIX", "nats.subject_prefix"),
    ("NATS_URL", "nats.url"),
    ("OMDB_API_KEY", "metadata.omdb_api_key"),
    (
        "POSTER_FETCH_TIMEOUT_SECONDS",
        "metadata.poster_fetch_timeout_seconds",
    ),
    ("RATE_LIMIT", "server.rate_limit"),
    ("REFRESH_TTL_SECONDS", "auth.refresh_ttl_seconds"),
    ("SECURE_COOKIES", "server.secure_cookies"),
    ("TMDB_API_KEY", "metadata.tmdb_api_key"),
    ("WRAPUP_BG_DIR", "wrapup.bg_dir"),
    ("WRAPUP_FONT_PATH", "wrapup.font_path"),
    ("WRAPUP_LOGO_PATH", "wrapup.logo_path"),
];

struct LegacyEnv;

impl Provider for LegacyEnv {
    fn metadata(&self) -> Metadata {
        Metadata::named("environment variables")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let mut sections: Dict = Dict::new();
        for (variable, key) in LEGACY_ENV {
            let Some(raw) = std::env::var_os(variable) else {
                continue;
            };
            let value = raw
                .into_string()
                .map_err(|_| figment::Error::from(format!("{variable} is not valid unicode")))?;
            if value.trim().is_empty() {
                continue;
            }
            let (section, field) = key.split_once('.').expect("legacy keys are section.field");
            let entry = sections
                .entry(section.to_owned())
                .or_insert_with(|| Value::from(Dict::new()));
            if let Value::Dict(_, dict) = entry {
                dict.insert(field.to_owned(), Value::from(value));
            }
        }
        Ok(Profile::Default.collect(sections))
    }
}

pub fn load() -> Result<AppConfig, ConfigError> {
    let named_file = std::env::var(CONFIG_FILE_VARIABLE).ok();
    let path = named_file.as_deref().unwrap_or(DEFAULT_CONFIG_FILE);

    if named_file.is_some() && !Path::new(path).exists() {
        return Err(ConfigError::NamedFileMissing(path.to_owned()));
    }

    let figment = Figment::new().merge(Toml::file(path)).merge(LegacyEnv);

    figment
        .data()
        .map_err(|e| ConfigError::Unreadable(Box::new(e)))?;
    if figment.find_value("database.url").is_err() {
        return Err(ConfigError::Missing("DATABASE_URL"));
    }

    let mut config: AppConfig = figment.extract().map_err(into_config_error)?;
    config.server.base_url = config.server.base_url.trim_end_matches('/').to_owned();

    announce(path);

    Ok(config)
}

fn into_config_error(error: figment::Error) -> ConfigError {
    let key = error.path.join(".");
    match LEGACY_ENV.iter().find(|(_, k)| *k == key) {
        Some((variable, _)) if std::env::var_os(variable).is_some() => ConfigError::Invalid {
            variable: (*variable).to_owned(),
            reason: error.kind.to_string(),
        },
        _ => ConfigError::Unreadable(Box::new(error)),
    }
}

fn announce(path: &str) {
    if Path::new(path).exists() {
        tracing::info!(path, "loaded configuration from file");
    } else {
        tracing::info!("no configuration file found, using defaults");
    }

    let overriding: Vec<&str> = LEGACY_ENV
        .iter()
        .map(|(variable, _)| *variable)
        .filter(|variable| std::env::var_os(variable).is_some())
        .collect();
    if !overriding.is_empty() {
        tracing::info!(variables = ?overriding, "configuration overridden from the environment");
    }
}
