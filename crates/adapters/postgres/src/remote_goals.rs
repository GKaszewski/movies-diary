use async_trait::async_trait;
use chrono::TimeZone;
use domain::{errors::DomainError, models::RemoteGoalEntry, ports::RemoteGoalRepository};
use sqlx::{PgPool, Row};

pub struct PostgresRemoteGoalRepository {
    pool: PgPool,
}

impl PostgresRemoteGoalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl RemoteGoalRepository for PostgresRemoteGoalRepository {
    async fn save(&self, entry: RemoteGoalEntry) -> Result<(), DomainError> {
        let received = entry.received_at.format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "INSERT INTO remote_goals \
             (ap_id, actor_url, year, target_count, current_count, received_at) \
             VALUES ($1, $2, $3, $4, $5, $6::timestamptz) \
             ON CONFLICT (ap_id) DO UPDATE SET \
             target_count = $4, current_count = $5",
        )
        .bind(&entry.ap_id)
        .bind(&entry.actor_url)
        .bind(entry.year as i64)
        .bind(entry.target_count as i64)
        .bind(entry.current_count as i64)
        .bind(&received)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn update_by_ap_id(
        &self,
        ap_id: &str,
        target: u32,
        current: u32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE remote_goals SET target_count = $1, current_count = $2 WHERE ap_id = $3",
        )
        .bind(target as i64)
        .bind(current as i64)
        .bind(ap_id)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM remote_goals WHERE ap_id = $1 AND actor_url = $2")
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;

        Ok(())
    }

    async fn get_by_actor_url(&self, actor_url: &str) -> Result<Vec<RemoteGoalEntry>, DomainError> {
        let rows = sqlx::query(
            "SELECT ap_id, actor_url, year, target_count, current_count, \
             to_char(received_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS received_at \
             FROM remote_goals WHERE actor_url = $1 ORDER BY year DESC",
        )
        .bind(actor_url)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.iter()
            .map(|r| {
                let year: i64 = r.try_get("year").unwrap_or(0);
                let target: i64 = r.try_get("target_count").unwrap_or(0);
                let current: i64 = r.try_get("current_count").unwrap_or(0);
                let received_str: String = r.try_get("received_at").unwrap_or_default();
                let received_at =
                    chrono::NaiveDateTime::parse_from_str(&received_str, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| chrono::Utc.from_utc_datetime(&ndt))
                        .unwrap_or_else(|_| chrono::Utc::now());

                Ok(RemoteGoalEntry {
                    ap_id: r.try_get("ap_id").unwrap_or_default(),
                    actor_url: r.try_get("actor_url").unwrap_or_default(),
                    year: year as u16,
                    target_count: target as u32,
                    current_count: current as u32,
                    received_at,
                })
            })
            .collect()
    }
}
