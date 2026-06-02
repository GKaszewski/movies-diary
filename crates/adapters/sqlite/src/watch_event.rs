use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{PersistedWatchEvent, WatchEvent, WatchEventSource, WatchEventStatus, WebhookToken},
    ports::{WatchEventRepository, WebhookTokenRepository},
    value_objects::{MovieId, UserId, WatchEventId, WebhookTokenId},
};
use sqlx::{Row, SqlitePool};

use crate::models::datetime_to_str;

fn map_err(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database operation failed".into())
}

fn parse_uuid(s: &str) -> Result<uuid::Uuid, DomainError> {
    s.parse()
        .map_err(|_| DomainError::InfrastructureError(format!("invalid UUID: {s}")))
}

fn parse_datetime(s: &str) -> Result<chrono::NaiveDateTime, DomainError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| DomainError::InfrastructureError(format!("invalid datetime: {s}")))
}

// ── WatchEventRepository ──────────────────────────────────────────────────────

pub struct SqliteWatchEventRepository {
    pool: SqlitePool,
}

impl SqliteWatchEventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WatchEventRepository for SqliteWatchEventRepository {
    async fn save(&self, event: &WatchEvent) -> Result<(), DomainError> {
        let id = event.id().value().to_string();
        let user_id = event.user_id().value().to_string();
        let movie_id = event.movie_id().map(|m| m.value().to_string());
        let source = event.source().to_string();
        let watched_at = datetime_to_str(event.watched_at());
        let status = event.status().to_string();
        let created_at = datetime_to_str(event.created_at());

        sqlx::query(
            "INSERT INTO watch_events \
             (id, user_id, movie_id, title, year, external_metadata_id, source, watched_at, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&movie_id)
        .bind(event.title())
        .bind(event.year().map(|y| y as i64))
        .bind(event.external_metadata_id())
        .bind(&source)
        .bind(&watched_at)
        .bind(&status)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn update_status(
        &self,
        id: &WatchEventId,
        status: WatchEventStatus,
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let status_str = status.to_string();

        sqlx::query("UPDATE watch_events SET status = ? WHERE id = ?")
            .bind(&status_str)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(())
    }

    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events \
             WHERE user_id = ? AND status = 'pending' \
             ORDER BY watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.iter().map(row_to_watch_event).collect()
    }

    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError> {
        let id_str = id.value().to_string();

        let row = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.as_ref().map(row_to_watch_event).transpose()
    }

    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id.value().to_string());
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        rows.iter().map(row_to_watch_event).collect()
    }

    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        status: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE watch_events SET status = ? WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&sql).bind(status.to_string());
        for id in ids {
            q = q.bind(id.value().to_string());
        }
        let result = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(result.rows_affected())
    }

    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let after_str = datetime_to_str(&after);

        let count: i64 = sqlx::query(
            "SELECT COUNT(*) FROM watch_events \
             WHERE user_id = ? AND external_metadata_id = ? AND created_at > ?",
        )
        .bind(&uid)
        .bind(external_id)
        .bind(&after_str)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?
        .try_get(0)
        .unwrap_or(0);

        Ok(count > 0)
    }

    async fn delete_non_pending_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let before_str = datetime_to_str(&before);
        let result =
            sqlx::query("DELETE FROM watch_events WHERE status != 'pending' AND created_at < ?")
                .bind(&before_str)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(result.rows_affected())
    }
}

fn row_to_watch_event(row: &sqlx::sqlite::SqliteRow) -> Result<WatchEvent, DomainError> {
    let id_str: &str = row.try_get("id").map_err(map_err)?;
    let user_id_str: &str = row.try_get("user_id").map_err(map_err)?;
    let movie_id_str: Option<&str> = row.try_get("movie_id").map_err(map_err)?;
    let title: String = row.try_get("title").map_err(map_err)?;
    let year: Option<i64> = row.try_get("year").map_err(map_err)?;
    let ext_id: Option<String> = row.try_get("external_metadata_id").map_err(map_err)?;
    let source_str: String = row.try_get("source").map_err(map_err)?;
    let watched_at_str: String = row.try_get("watched_at").map_err(map_err)?;
    let status_str: String = row.try_get("status").map_err(map_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_err)?;

    let source: WatchEventSource = source_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;
    let status: WatchEventStatus = status_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let movie_id = movie_id_str
        .map(parse_uuid)
        .transpose()?
        .map(MovieId::from_uuid);

    Ok(WatchEvent::from_persistence(PersistedWatchEvent {
        id: WatchEventId::from_uuid(parse_uuid(id_str)?),
        user_id: UserId::from_uuid(parse_uuid(user_id_str)?),
        movie_id,
        title,
        year: year.map(|y| y as u16),
        external_metadata_id: ext_id,
        source,
        watched_at: parse_datetime(&watched_at_str)?,
        status,
        created_at: parse_datetime(&created_at_str)?,
    }))
}

// ── WebhookTokenRepository ────────────────────────────────────────────────────

pub struct SqliteWebhookTokenRepository {
    pool: SqlitePool,
}

impl SqliteWebhookTokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookTokenRepository for SqliteWebhookTokenRepository {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError> {
        let id = token.id().value().to_string();
        let user_id = token.user_id().value().to_string();
        let provider = token.provider().to_string();
        let created_at = datetime_to_str(token.created_at());
        let last_used = token.last_used_at().map(datetime_to_str);

        sqlx::query(
            "INSERT INTO webhook_tokens \
             (id, user_id, token_hash, provider, label, created_at, last_used_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(token.token_hash())
        .bind(&provider)
        .bind(token.label())
        .bind(&created_at)
        .bind(&last_used)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, token_hash, provider, label, created_at, last_used_at \
             FROM webhook_tokens WHERE token_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.as_ref().map(row_to_webhook_token).transpose()
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, token_hash, provider, label, created_at, last_used_at \
             FROM webhook_tokens WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        rows.iter().map(row_to_webhook_token).collect()
    }

    async fn delete(&self, id: &WebhookTokenId, user_id: &UserId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let uid = user_id.value().to_string();

        let result = sqlx::query("DELETE FROM webhook_tokens WHERE id = ? AND user_id = ?")
            .bind(&id_str)
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Webhook token {id_str}")));
        }
        Ok(())
    }

    async fn touch_last_used(&self, id: &WebhookTokenId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let now = datetime_to_str(&chrono::Utc::now().naive_utc());

        sqlx::query("UPDATE webhook_tokens SET last_used_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;

        Ok(())
    }
}

fn row_to_webhook_token(row: &sqlx::sqlite::SqliteRow) -> Result<WebhookToken, DomainError> {
    let id_str: &str = row.try_get("id").map_err(map_err)?;
    let user_id_str: &str = row.try_get("user_id").map_err(map_err)?;
    let token_hash: String = row.try_get("token_hash").map_err(map_err)?;
    let provider_str: String = row.try_get("provider").map_err(map_err)?;
    let label: Option<String> = row.try_get("label").map_err(map_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_err)?;
    let last_used_str: Option<String> = row.try_get("last_used_at").map_err(map_err)?;

    let provider: WatchEventSource = provider_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let last_used = last_used_str.map(|s| parse_datetime(&s)).transpose()?;

    Ok(WebhookToken::from_persistence(
        WebhookTokenId::from_uuid(parse_uuid(id_str)?),
        UserId::from_uuid(parse_uuid(user_id_str)?),
        token_hash,
        provider,
        label,
        parse_datetime(&created_at_str)?,
        last_used,
    ))
}
