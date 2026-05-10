pub(crate) async fn run(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("postgres-event-queue migration failed: {e}"))
}
