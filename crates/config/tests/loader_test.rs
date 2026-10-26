#![allow(clippy::result_large_err)]

use config::{ConfigError, load};
use figment::Jail;

fn jail_with(jail: &mut Jail, vars: &[(&str, &str)]) {
    jail.clear_env();
    jail.set_env("DATABASE_URL", "sqlite://test.db");
    for (name, value) in vars {
        jail.set_env(name, value);
    }
}

macro_rules! env_sets {
    ($test:ident, $var:literal, $value:literal, |$c:ident| $assert:expr) => {
        #[test]
        fn $test() {
            Jail::expect_with(|jail| {
                jail_with(jail, &[($var, $value)]);
                let $c = load().expect("config loads");
                assert!($assert, "{} not applied", $var);
                Ok(())
            });
        }
    };
}

env_sets!(
    allow_registration_env_sets_allow_registration,
    "ALLOW_REGISTRATION",
    "true",
    |c| c.server.allow_registration
);
env_sets!(
    base_url_env_sets_base_url,
    "BASE_URL",
    "https://x.example/",
    |c| c.server.base_url == "https://x.example"
);
env_sets!(
    cors_origins_env_sets_cors_origins,
    "CORS_ORIGINS",
    "https://a.example,https://b.example",
    |c| c.server.cors_origins == "https://a.example,https://b.example"
);
env_sets!(
    database_url_env_sets_url,
    "DATABASE_URL",
    "sqlite://other.db",
    |c| c.database.url == "sqlite://other.db"
);
env_sets!(
    event_bus_backend_env_sets_bus_backend,
    "EVENT_BUS_BACKEND",
    "nats",
    |c| c.events.bus_backend == "nats"
);
env_sets!(
    event_channel_buffer_env_sets_channel_buffer,
    "EVENT_CHANNEL_BUFFER",
    "7",
    |c| c.events.channel_buffer == 7
);
env_sets!(
    event_queue_batch_size_env_sets_queue_batch_size,
    "EVENT_QUEUE_BATCH_SIZE",
    "11",
    |c| c.events.queue_batch_size == 11
);
env_sets!(
    event_queue_max_attempts_env_sets_queue_max_attempts,
    "EVENT_QUEUE_MAX_ATTEMPTS",
    "9",
    |c| c.events.queue_max_attempts == 9
);
env_sets!(
    event_queue_poll_interval_ms_env_sets_queue_poll_interval_ms,
    "EVENT_QUEUE_POLL_INTERVAL_MS",
    "250",
    |c| c.events.queue_poll_interval_ms == 250
);
env_sets!(
    federation_debug_env_sets_debug,
    "FEDERATION_DEBUG",
    "1",
    |c| c.federation.debug
);
env_sets!(host_env_sets_host, "HOST", "127.0.0.1", |c| c.server.host
    == "127.0.0.1");
env_sets!(port_env_sets_port, "PORT", "8080", |c| c.server.port
    == 8080);
env_sets!(
    image_conversion_enabled_env_sets_enabled,
    "IMAGE_CONVERSION_ENABLED",
    "true",
    |c| c.image_conversion.enabled
);
env_sets!(
    image_conversion_format_env_sets_format,
    "IMAGE_CONVERSION_FORMAT",
    "webp",
    |c| c.image_conversion.format.as_deref() == Some("webp")
);
env_sets!(
    image_storage_backend_env_sets_backend,
    "IMAGE_STORAGE_BACKEND",
    "s3",
    |c| c.storage.backend == "s3"
);
env_sets!(
    image_storage_path_env_sets_path,
    "IMAGE_STORAGE_PATH",
    "/data/img",
    |c| c.storage.path == "/data/img"
);
env_sets!(
    jwt_secret_env_sets_jwt_secret,
    "JWT_SECRET",
    "s3cret",
    |c| c.auth.jwt_secret.as_deref() == Some("s3cret")
);
env_sets!(
    jwt_ttl_seconds_env_sets_jwt_ttl_seconds,
    "JWT_TTL_SECONDS",
    "60",
    |c| c.auth.jwt_ttl_seconds == 60
);
env_sets!(
    minio_access_key_id_env_sets_access_key_id,
    "MINIO_ACCESS_KEY_ID",
    "ak",
    |c| c.storage.access_key_id.as_deref() == Some("ak")
);
env_sets!(minio_bucket_env_sets_bucket, "MINIO_BUCKET", "b", |c| c
    .storage
    .bucket
    .as_deref()
    == Some("b"));
env_sets!(
    minio_endpoint_env_sets_endpoint,
    "MINIO_ENDPOINT",
    "http://minio:9000",
    |c| c.storage.endpoint.as_deref() == Some("http://minio:9000")
);
env_sets!(minio_region_env_sets_region, "MINIO_REGION", "eu", |c| c
    .storage
    .region
    == "eu");
env_sets!(
    minio_secret_access_key_env_sets_secret_access_key,
    "MINIO_SECRET_ACCESS_KEY",
    "sk",
    |c| c.storage.secret_access_key.as_deref() == Some("sk")
);
env_sets!(
    nats_consumer_name_env_sets_consumer_name,
    "NATS_CONSUMER_NAME",
    "w2",
    |c| c.nats.consumer_name == "w2"
);
env_sets!(nats_mode_env_sets_mode, "NATS_MODE", "core", |c| c
    .nats
    .mode
    == "core");
env_sets!(
    nats_stream_name_env_sets_stream_name,
    "NATS_STREAM_NAME",
    "S",
    |c| c.nats.stream_name == "S"
);
env_sets!(
    nats_subject_prefix_env_sets_subject_prefix,
    "NATS_SUBJECT_PREFIX",
    "p.e",
    |c| c.nats.subject_prefix == "p.e"
);
env_sets!(nats_url_env_sets_url, "NATS_URL", "nats://n:4222", |c| c
    .nats
    .url
    .as_deref()
    == Some("nats://n:4222"));
env_sets!(
    omdb_api_key_env_sets_omdb_api_key,
    "OMDB_API_KEY",
    "ok",
    |c| c.metadata.omdb_api_key.as_deref() == Some("ok")
);
env_sets!(
    poster_fetch_timeout_seconds_env_sets_poster_fetch_timeout_seconds,
    "POSTER_FETCH_TIMEOUT_SECONDS",
    "5",
    |c| c.metadata.poster_fetch_timeout_seconds == 5
);
env_sets!(rate_limit_env_sets_rate_limit, "RATE_LIMIT", "120", |c| c
    .server
    .rate_limit
    == 120);
env_sets!(
    refresh_ttl_seconds_env_sets_refresh_ttl_seconds,
    "REFRESH_TTL_SECONDS",
    "99",
    |c| c.auth.refresh_ttl_seconds == 99
);
env_sets!(
    secure_cookies_env_sets_secure_cookies,
    "SECURE_COOKIES",
    "true",
    |c| c.server.secure_cookies
);
env_sets!(
    tmdb_api_key_env_sets_tmdb_api_key,
    "TMDB_API_KEY",
    "tk",
    |c| c.metadata.tmdb_api_key.as_deref() == Some("tk")
);
env_sets!(wrapup_bg_dir_env_sets_bg_dir, "WRAPUP_BG_DIR", "/bg", |c| c
    .wrapup
    .bg_dir
    .as_deref()
    == Some("/bg"));
env_sets!(
    wrapup_font_path_env_sets_font_path,
    "WRAPUP_FONT_PATH",
    "/f.ttf",
    |c| c.wrapup.font_path.as_deref() == Some("/f.ttf")
);
env_sets!(
    wrapup_logo_path_env_sets_logo_path,
    "WRAPUP_LOGO_PATH",
    "/l.png",
    |c| c.wrapup.logo_path.as_deref() == Some("/l.png")
);

#[test]
fn numeric_looking_string_values_stay_strings() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("JWT_SECRET", "12345"), ("TMDB_API_KEY", "1")]);
        let c = load().expect("config loads");
        assert_eq!(c.auth.jwt_secret.as_deref(), Some("12345"));
        assert_eq!(c.metadata.tmdb_api_key.as_deref(), Some("1"));
        Ok(())
    });
}

#[test]
fn defaults_match_the_previous_env_readers() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[]);
        let c = load().expect("config loads");
        assert!(!c.server.allow_registration);
        assert_eq!(c.server.base_url, "http://localhost:3000");
        assert_eq!(c.server.cors_origins, "");
        assert_eq!(c.server.host, "0.0.0.0");
        assert_eq!(c.server.port, 3000);
        assert_eq!(c.server.rate_limit, 60);
        assert!(!c.server.secure_cookies);
        assert_eq!(c.database.url, "sqlite://test.db");
        assert_eq!(c.auth.jwt_secret, None);
        assert_eq!(c.auth.jwt_ttl_seconds, 900);
        assert_eq!(c.auth.refresh_ttl_seconds, 2_592_000);
        assert_eq!(c.events.bus_backend, "db");
        assert_eq!(c.events.channel_buffer, 128);
        assert_eq!(c.events.queue_poll_interval_ms, 500);
        assert_eq!(c.events.queue_batch_size, 10);
        assert_eq!(c.events.queue_max_attempts, 5);
        assert_eq!(c.nats.url, None);
        assert_eq!(c.nats.mode, "jetstream");
        assert_eq!(c.nats.subject_prefix, "movies-diary.events");
        assert_eq!(c.nats.stream_name, "MOVIES_DIARY_EVENTS");
        assert_eq!(c.nats.consumer_name, "worker");
        assert_eq!(c.storage.backend, "local");
        assert_eq!(c.storage.path, "./images");
        assert_eq!(c.storage.endpoint, None);
        assert_eq!(c.storage.access_key_id, None);
        assert_eq!(c.storage.secret_access_key, None);
        assert_eq!(c.storage.bucket, None);
        assert_eq!(c.storage.region, "minio");
        assert!(!c.image_conversion.enabled);
        assert_eq!(c.image_conversion.format, None);
        assert_eq!(c.metadata.tmdb_api_key, None);
        assert_eq!(c.metadata.omdb_api_key, None);
        assert_eq!(c.metadata.poster_fetch_timeout_seconds, 30);
        assert!(!c.federation.debug);
        assert_eq!(c.wrapup.font_path, None);
        assert_eq!(c.wrapup.logo_path, None);
        assert_eq!(c.wrapup.bg_dir, None);
        Ok(())
    });
}

#[test]
fn toml_file_is_overridden_by_env() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("PORT", "9000")]);
        jail.create_file(
            "config.toml",
            "[server]\nport = 1234\nhost = \"10.0.0.1\"\n[auth]\njwt_secret = \"from-file\"\n",
        )?;
        let c = load().expect("config loads");
        assert_eq!(c.server.port, 9000, "env wins over file");
        assert_eq!(c.server.host, "10.0.0.1", "file wins over default");
        assert_eq!(c.auth.jwt_secret.as_deref(), Some("from-file"));
        Ok(())
    });
}

#[test]
fn config_file_env_names_an_alternative_file() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("CONFIG_FILE", "alt.toml")]);
        jail.create_file("config.toml", "[server]\nport = 1\n")?;
        jail.create_file("alt.toml", "[server]\nport = 2\n")?;
        assert_eq!(load().expect("config loads").server.port, 2);
        Ok(())
    });
}

#[test]
fn missing_named_config_file_is_an_error() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("CONFIG_FILE", "nope.toml")]);
        let err = load().expect_err("named file is missing");
        assert!(matches!(err, ConfigError::NamedFileMissing(_)), "{err}");
        Ok(())
    });
}

#[test]
fn missing_database_url_is_an_error_naming_the_variable() {
    Jail::expect_with(|jail| {
        jail.clear_env();
        let err = load().expect_err("DATABASE_URL is required");
        assert!(err.to_string().contains("DATABASE_URL"), "{err}");
        Ok(())
    });
}

#[test]
fn malformed_numeric_env_value_fails_naming_the_variable() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("RATE_LIMIT", "abc")]);
        let err = load().expect_err("RATE_LIMIT=abc is malformed");
        assert!(err.to_string().contains("RATE_LIMIT"), "{err}");
        Ok(())
    });
}

#[test]
fn empty_env_value_falls_back_to_the_default() {
    Jail::expect_with(|jail| {
        jail_with(
            jail,
            &[
                ("RATE_LIMIT", ""),
                ("PORT", "  "),
                ("TMDB_API_KEY", ""),
                ("NATS_URL", ""),
            ],
        );
        let c = load().expect("empty values are unset, not errors");
        assert_eq!(c.server.rate_limit, 60);
        assert_eq!(c.server.port, 3000);
        assert!(c.metadata.tmdb_api_key.is_none());
        assert!(c.nats.url.is_none());
        Ok(())
    });
}

#[test]
fn allow_registration_accepts_true_and_1_only() {
    for (value, expected) in [
        ("true", true),
        ("1", true),
        ("false", false),
        ("0", false),
        ("yes", false),
        ("TRUE", false),
    ] {
        Jail::expect_with(|jail| {
            jail_with(jail, &[("ALLOW_REGISTRATION", value)]);
            let c = load().expect("config loads");
            assert_eq!(c.server.allow_registration, expected, "value {value:?}");
            Ok(())
        });
    }
}

#[test]
fn secure_cookies_and_image_conversion_accept_true_only() {
    Jail::expect_with(|jail| {
        jail_with(
            jail,
            &[("SECURE_COOKIES", "1"), ("IMAGE_CONVERSION_ENABLED", "1")],
        );
        let c = load().expect("config loads");
        assert!(!c.server.secure_cookies);
        assert!(!c.image_conversion.enabled);
        Ok(())
    });
}

#[test]
fn malformed_config_file_reports_the_file_not_a_missing_database_url() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[]);
        jail.create_file("config.toml", "[server\nport = = 1\n")?;
        let err = load().expect_err("config.toml is malformed");
        assert!(matches!(err, ConfigError::Unreadable(_)), "{err}");
        assert!(!err.to_string().contains("DATABASE_URL"), "{err}");
        Ok(())
    });
}

#[test]
fn empty_database_url_is_reported_missing() {
    Jail::expect_with(|jail| {
        jail_with(jail, &[("DATABASE_URL", "")]);
        let err = load().expect_err("empty DATABASE_URL is unset");
        assert!(matches!(err, ConfigError::Missing("DATABASE_URL")), "{err}");
        Ok(())
    });
}
