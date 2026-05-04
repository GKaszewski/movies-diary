use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::Movie,
    ports::{EventPublisher, MetadataClient, PosterFetcherClient, PosterStorage},
    value_objects::{ExternalMetadataId, MovieId, PosterPath, PosterUrl},
};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{config::AppConfig, context::AppContext};
use auth::{AuthConfig, Argon2PasswordHasher, JwtAuthService};
use sqlite::{SqliteMovieRepository, SqliteUserRepository};
use template_askama::AskamaHtmlRenderer;

use presentation::{routes, state::AppState};

struct StubMetadataClient;

#[async_trait]
impl MetadataClient for StubMetadataClient {
    async fn fetch_movie_metadata(&self, _id: &ExternalMetadataId) -> Result<Movie, DomainError> {
        Err(DomainError::InfrastructureError(
            "metadata client not implemented".into(),
        ))
    }

    async fn get_poster_url(
        &self,
        _id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        Err(DomainError::InfrastructureError(
            "metadata client not implemented".into(),
        ))
    }
}

struct StubPosterFetcher;

#[async_trait]
impl PosterFetcherClient for StubPosterFetcher {
    async fn fetch_poster_bytes(&self, _url: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::InfrastructureError(
            "poster fetcher not implemented".into(),
        ))
    }
}

struct StubPosterStorage;

#[async_trait]
impl PosterStorage for StubPosterStorage {
    async fn store_poster(
        &self,
        _movie_id: &MovieId,
        _bytes: &[u8],
    ) -> Result<PosterPath, DomainError> {
        Err(DomainError::InfrastructureError(
            "poster storage not implemented".into(),
        ))
    }

    async fn get_poster(&self, _path: &PosterPath) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::InfrastructureError(
            "poster storage not implemented".into(),
        ))
    }
}

struct StubEventPublisher;

#[async_trait]
impl EventPublisher for StubEventPublisher {
    async fn publish(&self, _event: &DomainEvent) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let state = wire_dependencies()
        .await
        .context("Failed to wire dependencies")?;

    let app = routes::build_router(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<AppState> {
    let auth_config = AuthConfig::from_env()?;
    let app_config = AppConfig::from_env();

    let pool = SqlitePool::connect("sqlite://reviews.db")
        .await
        .context("Failed to connect to SQLite database")?;

    let movie_repo = SqliteMovieRepository::new(pool.clone());
    movie_repo
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Database migration failed")?;

    let user_repo = SqliteUserRepository::new(pool);

    let app_ctx = AppContext {
        repository: Arc::new(movie_repo),
        metadata_client: Arc::new(StubMetadataClient),
        poster_fetcher: Arc::new(StubPosterFetcher),
        poster_storage: Arc::new(StubPosterStorage),
        event_publisher: Arc::new(StubEventPublisher),
        auth_service: Arc::new(JwtAuthService::new(auth_config)),
        password_hasher: Arc::new(Argon2PasswordHasher),
        user_repository: Arc::new(user_repo),
        config: app_config,
    };

    Ok(AppState {
        app_ctx,
        html_renderer: Arc::new(AskamaHtmlRenderer::new()),
    })
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "presentation=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
