use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    bootstrap::init_tracing("worker=info,bootstrap=info,config=info,application=info,k_ap=info");

    let config = config::load().context("Failed to load configuration")?;
    let app = bootstrap::build_worker(&config).await?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = shutdown_tx.send(true);
    });

    tracing::info!("worker started");
    app.run(shutdown_rx).await
}
