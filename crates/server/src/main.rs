use anyhow::Context;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    bootstrap::init_tracing("server=debug,bootstrap=debug,config=info,tower_http=debug");

    let config = config::load().context("Failed to load configuration")?;
    let app = bootstrap::build_server(&config).await?;

    let listener = TcpListener::bind(app.addr).await?;
    tracing::info!("Listening on {}", app.addr);
    axum::serve(
        listener,
        app.router
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
