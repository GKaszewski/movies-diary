pub(crate) async fn run(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("postgres-event-queue migration failed: {e}"))
}
