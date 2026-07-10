use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::{BlockedDomain, BlocklistRepository};
use sqlx::Row;

use adapter_common::datetime_to_str;
use super::SqliteFederationRepository;

#[async_trait]
impl BlocklistRepository for SqliteFederationRepository {
    async fn add_blocked_domain(&self, domain: &str, reason: Option<&str>) -> Result<()> {
        let now = Utc::now().naive_utc();
        let ts = datetime_to_str(&now);
        sqlx::query(
            "INSERT INTO blocked_domains (domain, reason, blocked_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(domain) DO UPDATE SET reason = excluded.reason",
        )
        .bind(domain)
        .bind(reason)
        .bind(&ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_blocked_domain(&self, domain: &str) -> Result<()> {
        sqlx::query("DELETE FROM blocked_domains WHERE domain = ?1")
            .bind(domain)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_blocked_domains(&self) -> Result<Vec<BlockedDomain>> {
        let rows = sqlx::query(
            "SELECT domain, reason, blocked_at FROM blocked_domains ORDER BY blocked_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| BlockedDomain {
                domain: r.get("domain"),
                reason: r.get("reason"),
                blocked_at: r.get("blocked_at"),
            })
            .collect())
    }

    async fn is_domain_blocked(&self, domain: &str) -> Result<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blocked_domains WHERE domain = ?1")
                .bind(domain)
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }

    async fn add_blocked_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        let ts = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query(
            "INSERT OR IGNORE INTO blocked_actors (local_user_id, remote_actor_url, blocked_at) VALUES (?1, ?2, ?3)",
        ).bind(&uid).bind(actor_url).bind(&ts).execute(&self.pool).await?;
        Ok(())
    }

    async fn remove_blocked_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query(
            "DELETE FROM blocked_actors WHERE local_user_id = ?1 AND remote_actor_url = ?2",
        )
        .bind(&uid)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_blocked_actors(&self, local_user_id: uuid::Uuid) -> Result<Vec<String>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT remote_actor_url FROM blocked_actors WHERE local_user_id = ?1 ORDER BY blocked_at DESC",
        ).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| r.get::<String, _>("remote_actor_url"))
            .collect())
    }

    async fn is_actor_blocked(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<bool> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blocked_actors WHERE local_user_id = ?1 AND remote_actor_url = ?2",
        ).bind(&uid).bind(actor_url).fetch_one(&self.pool).await?;
        Ok(count > 0)
    }
}
