use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::ActivityRepository;

use super::SqliteFederationRepository;
use adapter_common::datetime_to_str;

#[async_trait]
impl ActivityRepository for SqliteFederationRepository {
    async fn is_activity_processed(&self, activity_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ap_activities WHERE id = ?1")
            .bind(activity_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    async fn mark_activity_processed(&self, activity_id: &str) -> Result<()> {
        let ts = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query("INSERT OR IGNORE INTO ap_activities (id, processed_at) VALUES (?1, ?2)")
            .bind(activity_id)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
