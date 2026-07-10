use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{PersistedWatchEvent, WatchEvent, WatchEventSource, WatchEventStatus, WebhookToken},
    ports::{WatchEventCommand, WatchEventQuery, WebhookTokenRepository},
    value_objects::{MovieId, UserId, WatchEventId, WebhookTokenId},
};
use sqlx::{PgPool, Row};

use adapter_common::{parse_datetime, parse_uuid};


// ── WatchEventRepository ──────────────────────────────────────────────────────

pub struct PostgresWatchEventRepository {
    pool: PgPool,
}

impl PostgresWatchEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WatchEventCommand for PostgresWatchEventRepository {
    async fn save(&self, event: &WatchEvent) -> Result<(), DomainError> {
        let id = event.id().value().to_string();
        let user_id = event.user_id().value().to_string();
        let movie_id = event.movie_id().map(|m| m.value().to_string());
        let source = event.source().to_string();
        let status = event.status().to_string();

        sqlx::query(
            "INSERT INTO watch_events \
             (id, user_id, movie_id, title, year, external_metadata_id, source, watched_at, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&movie_id)
        .bind(event.title())
        .bind(event.year().map(|y| y as i32))
        .bind(event.external_metadata_id())
        .bind(&source)
        .bind(event.watched_at())
        .bind(&status)
        .bind(event.created_at())
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn update_status(
        &self,
        id: &WatchEventId,
        status: WatchEventStatus,
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let status_str = status.to_string();

        sqlx::query("UPDATE watch_events SET status = $1 WHERE id = $2")
            .bind(&status_str)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        status: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let id_strs: Vec<String> = ids.iter().map(|id| id.value().to_string()).collect();
        let status_str = status.to_string();
        let result = sqlx::query("UPDATE watch_events SET status = $1 WHERE id = ANY($2)")
            .bind(&status_str)
            .bind(&id_strs)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        Ok(result.rows_affected())
    }

    async fn delete_non_pending_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let result =
            sqlx::query("DELETE FROM watch_events WHERE status != 'pending' AND created_at < $1")
                .bind(before)
                .execute(&self.pool)
                .await
                .map_err(adapter_common::map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl WatchEventQuery for PostgresWatchEventRepository {
    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, \
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at, \
                    status, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM watch_events \
             WHERE user_id = $1 AND status = 'pending' \
             ORDER BY watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        rows.iter().map(row_to_watch_event).collect()
    }

    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError> {
        let id_str = id.value().to_string();

        let row = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, \
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at, \
                    status, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM watch_events WHERE id = $1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        row.as_ref().map(row_to_watch_event).transpose()
    }

    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let id_strs: Vec<String> = ids.iter().map(|id| id.value().to_string()).collect();
        let rows = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, \
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at, \
                    status, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM watch_events WHERE id = ANY($1)",
        )
        .bind(&id_strs)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        rows.iter().map(row_to_watch_event).collect()
    }

    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM watch_events \
             WHERE user_id = $1 AND external_metadata_id = $2 AND created_at > $3",
        )
        .bind(&uid)
        .bind(external_id)
        .bind(after)
        .fetch_one(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(count > 0)
    }
}

fn row_to_watch_event(row: &sqlx::postgres::PgRow) -> Result<WatchEvent, DomainError> {
    let id_str: String = row.try_get("id").map_err(adapter_common::map_sqlx_error)?;
    let user_id_str: String = row.try_get("user_id").map_err(adapter_common::map_sqlx_error)?;
    let movie_id_str: Option<String> = row.try_get("movie_id").map_err(adapter_common::map_sqlx_error)?;
    let title: String = row.try_get("title").map_err(adapter_common::map_sqlx_error)?;
    let year: Option<i32> = row.try_get("year").map_err(adapter_common::map_sqlx_error)?;
    let ext_id: Option<String> = row.try_get("external_metadata_id").map_err(adapter_common::map_sqlx_error)?;
    let source_str: String = row.try_get("source").map_err(adapter_common::map_sqlx_error)?;
    let watched_at_str: String = row.try_get("watched_at").map_err(adapter_common::map_sqlx_error)?;
    let status_str: String = row.try_get("status").map_err(adapter_common::map_sqlx_error)?;
    let created_at_str: String = row.try_get("created_at").map_err(adapter_common::map_sqlx_error)?;

    let source: WatchEventSource = source_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;
    let status: WatchEventStatus = status_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let movie_id = movie_id_str
        .as_deref()
        .map(parse_uuid)
        .transpose()?
        .map(MovieId::from_uuid);

    Ok(WatchEvent::from_persistence(PersistedWatchEvent {
        id: WatchEventId::from_uuid(parse_uuid(&id_str)?),
        user_id: UserId::from_uuid(parse_uuid(&user_id_str)?),
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

pub struct PostgresWebhookTokenRepository {
    pool: PgPool,
}

impl PostgresWebhookTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookTokenRepository for PostgresWebhookTokenRepository {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError> {
        let id = token.id().value().to_string();
        let user_id = token.user_id().value().to_string();
        let provider = token.provider().to_string();

        sqlx::query(
            "INSERT INTO webhook_tokens \
             (id, user_id, token_hash, provider, label, created_at, last_used_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(token.token_hash())
        .bind(&provider)
        .bind(token.label())
        .bind(token.created_at())
        .bind(token.last_used_at())
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError> {
        let row = sqlx::query(
            "SELECT id, user_id, token_hash, provider, label, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(last_used_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_used_at \
             FROM webhook_tokens WHERE token_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        row.as_ref().map(row_to_webhook_token).transpose()
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, token_hash, provider, label, \
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at, \
                    to_char(last_used_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_used_at \
             FROM webhook_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        rows.iter().map(row_to_webhook_token).collect()
    }

    async fn delete(&self, id: &WebhookTokenId, user_id: &UserId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let uid = user_id.value().to_string();

        let result = sqlx::query("DELETE FROM webhook_tokens WHERE id = $1 AND user_id = $2")
            .bind(&id_str)
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Webhook token {id_str}")));
        }
        Ok(())
    }

    async fn touch_last_used(&self, id: &WebhookTokenId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();

        sqlx::query("UPDATE webhook_tokens SET last_used_at = NOW() WHERE id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }
}

fn row_to_webhook_token(row: &sqlx::postgres::PgRow) -> Result<WebhookToken, DomainError> {
    let id_str: String = row.try_get("id").map_err(adapter_common::map_sqlx_error)?;
    let user_id_str: String = row.try_get("user_id").map_err(adapter_common::map_sqlx_error)?;
    let token_hash: String = row.try_get("token_hash").map_err(adapter_common::map_sqlx_error)?;
    let provider_str: String = row.try_get("provider").map_err(adapter_common::map_sqlx_error)?;
    let label: Option<String> = row.try_get("label").map_err(adapter_common::map_sqlx_error)?;
    let created_at_str: String = row.try_get("created_at").map_err(adapter_common::map_sqlx_error)?;
    let last_used_str: Option<String> = row.try_get("last_used_at").map_err(adapter_common::map_sqlx_error)?;

    let provider: WatchEventSource = provider_str
        .parse()
        .map_err(|e: String| DomainError::InfrastructureError(e))?;

    let last_used = last_used_str.as_deref().map(parse_datetime).transpose()?;

    Ok(WebhookToken::from_persistence(
        WebhookTokenId::from_uuid(parse_uuid(&id_str)?),
        UserId::from_uuid(parse_uuid(&user_id_str)?),
        token_hash,
        provider,
        label,
        parse_datetime(&created_at_str)?,
        last_used,
    ))
}
