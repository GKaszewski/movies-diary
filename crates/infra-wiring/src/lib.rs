pub mod config;

pub use config::{AppConfig, WrapUpConfig};

pub enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

#[derive(Clone, Copy)]
pub enum EventBusBackend {
    Db,
    #[cfg(feature = "nats")]
    Nats,
}

impl EventBusBackend {
    pub fn from_env() -> anyhow::Result<Self> {
        match std::env::var("EVENT_BUS_BACKEND")
            .unwrap_or_else(|_| "db".to_string())
            .as_str()
        {
            "db" => Ok(Self::Db),
            #[cfg(feature = "nats")]
            "nats" => Ok(Self::Nats),
            #[cfg(not(feature = "nats"))]
            "nats" => {
                anyhow::bail!("EVENT_BUS_BACKEND=nats requires the nats feature to be compiled in")
            }
            other => anyhow::bail!("unknown EVENT_BUS_BACKEND={other}, expected 'db' or 'nats'"),
        }
    }
}
