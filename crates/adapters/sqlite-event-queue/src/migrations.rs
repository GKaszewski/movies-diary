pub(crate) async fn run(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("sqlite-event-queue migration failed: {e}"))
}
