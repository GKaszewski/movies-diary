use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::wrapup::{WrapUpRecord, WrapUpReport, WrapUpStatus},
    ports::WrapUpRepository,
    value_objects::WrapUpId,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::shared::enqueue_events;
use rows::{row_to_record, status_to_str};

mod query;
mod rows;

pub use query::SqliteWrapUpStatsQuery;

// ── WrapUpRepository ─────────────────────────────────────────────────────────

pub struct SqliteWrapUpRepository {
    pool: SqlitePool,
}

impl SqliteWrapUpRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WrapUpRepository for SqliteWrapUpRepository {
    async fn create(
        &self,
        record: &WrapUpRecord,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id = record.id.value().to_string();
        let user_id = record.user_id.map(|u| u.to_string());
        let status = status_to_str(&record.status);
        let start = record.start_date.format("%Y-%m-%d").to_string();
        let end = record.end_date.format("%Y-%m-%d").to_string();
        let created = record.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = record
            .completed_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO wrap_up_records \
             (id, user_id, start_date, end_date, status, report_json, error_message, created_at, completed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&start)
        .bind(&end)
        .bind(status)
        .bind(record.report.as_ref().and_then(|r| serde_json::to_string(r).ok()))
        .bind(&record.error_message)
        .bind(&created)
        .bind(&completed)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let status_str = status_to_str(status);

        sqlx::query("UPDATE wrap_up_records SET status = ?, error_message = ? WHERE id = ?")
            .bind(status_str)
            .bind(error)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        Ok(())
    }

    async fn set_complete(
        &self,
        id: &WrapUpId,
        report: &WrapUpReport,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let json = serde_json::to_string(report)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let result = sqlx::query(
            "UPDATE wrap_up_records \
             SET status = 'ready', report_json = ?, completed_at = strftime('%Y-%m-%d %H:%M:%S', 'now') \
             WHERE id = ?",
        )
        .bind(&json)
        .bind(&id_str)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("wrapup record".into()));
        }
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn get_by_id(&self, id: &WrapUpId) -> Result<Option<WrapUpRecord>, DomainError> {
        let id_str = id.value().to_string();

        let row = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    created_at, completed_at \
             FROM wrap_up_records WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref().map(row_to_record).transpose()
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<WrapUpRecord>, DomainError> {
        let uid = user_id.to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    created_at, completed_at \
             FROM wrap_up_records WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        rows.iter().map(row_to_record).collect()
    }

    async fn list_global(&self) -> Result<Vec<WrapUpRecord>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    created_at, completed_at \
             FROM wrap_up_records WHERE user_id IS NULL ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        rows.iter().map(row_to_record).collect()
    }

    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<WrapUpRecord>, DomainError> {
        let uid = user_id.map(|u| u.to_string());
        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        let row = sqlx::query(
            "SELECT id, user_id, start_date, end_date, status, report_json, error_message, \
                    created_at, completed_at \
             FROM wrap_up_records \
             WHERE ((? IS NULL AND user_id IS NULL) OR user_id = ?) \
               AND start_date = ? AND end_date = ? \
             LIMIT 1",
        )
        .bind(&uid)
        .bind(&uid)
        .bind(&start_str)
        .bind(&end_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref().map(row_to_record).transpose()
    }

    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM wrap_up_records WHERE id = ?")
            .bind(id.value().to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        Ok(())
    }

    async fn delete_failed_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let before_str = before.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let result =
            sqlx::query("DELETE FROM wrap_up_records WHERE status = 'failed' AND created_at < ?")
                .bind(&before_str)
                .execute(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}
