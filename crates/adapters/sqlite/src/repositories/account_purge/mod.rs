use async_trait::async_trait;
use domain::{
    errors::DomainError, ports::AccountPurger, value_objects::InstanceIdentity,
    value_objects::UserId,
};
use sqlx::SqlitePool;

use crate::shared::map_sqlx_error;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

pub const PURGED_USER_TABLES: &[(&str, &str)] = &[
    ("reviews", "user_id"),
    ("watchlist_entries", "user_id"),
    ("goals", "user_id"),
    ("user_settings", "user_id"),
    ("user_profile_fields", "user_id"),
    ("webhook_tokens", "user_id"),
    ("watch_events", "user_id"),
    ("wrap_up_records", "user_id"),
    ("import_sessions", "user_id"),
    ("import_profiles", "user_id"),
    ("refresh_sessions", "user_id"),
    ("ap_followers", "local_user_id"),
    ("ap_following", "local_user_id"),
    ("blocked_actors", "local_user_id"),
];

pub const RETAINED_USER_TABLES: &[&str] = &["ap_local_actors"];

pub const URL_KEYED_USER_COLUMNS: &[(&str, &str)] = &[
    ("ap_announces", "object_url"),
    ("ap_announces", "actor_url"),
    ("ap_followers", "remote_actor_url"),
    ("ap_following", "remote_actor_url"),
];

pub const PAYLOAD_KEYED_USER_COLUMNS: &[(&str, &str)] = &[
    ("event_queue", "payload"),
    ("wrap_up_records", "report_json"),
];

pub struct SqliteAccountPurger {
    pool: SqlitePool,
    instance: InstanceIdentity,
}

impl SqliteAccountPurger {
    pub fn new(pool: SqlitePool, instance: InstanceIdentity) -> Self {
        Self { pool, instance }
    }
}

#[async_trait]
impl AccountPurger for SqliteAccountPurger {
    async fn is_purged(&self, user: &UserId) -> Result<bool, DomainError> {
        sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ? AND purged_at IS NOT NULL)",
        )
        .bind(user.value().to_string())
        .fetch_one(&self.pool)
        .await
        .map(|n| n != 0)
        .map_err(map_sqlx_error)
    }

    async fn owned_image_keys(&self, user: &UserId) -> Result<Vec<String>, DomainError> {
        let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT avatar_path, banner_path FROM users WHERE id = ? AND deleted_at IS NOT NULL",
        )
        .bind(user.value().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(row
            .map(|(avatar, banner)| avatar.into_iter().chain(banner).collect())
            .unwrap_or_default())
    }

    async fn purge(&self, user: &UserId) -> Result<(), DomainError> {
        let id = user.value().to_string();
        let actor_url = self.instance.actor_url_for(user);
        let base = self.instance.base_url().to_string();
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        let claimed = sqlx::query(
            "UPDATE users SET email = 'deleted-' || id || '@deleted.invalid',
                 password_hash = '!purged', display_name = NULL, bio = NULL,
                 also_known_as = NULL, purged_at = ?
             WHERE id = ? AND deleted_at IS NOT NULL AND purged_at IS NULL",
        )
        .bind(chrono::Utc::now().format(TIMESTAMP_FORMAT).to_string())
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .rows_affected();
        if claimed == 0 {
            return Ok(());
        }

        sqlx::query(
            "DELETE FROM ap_announces
             WHERE object_url IN (
                     SELECT ?1 || '/reviews/' || id FROM reviews
                     WHERE user_id = ?2 AND remote_actor_url IS NULL)
                OR substr(object_url, 1, length(?3)) = ?3
                OR actor_url = ?4",
        )
        .bind(&base)
        .bind(&id)
        .bind(format!("{base}/users/{id}/"))
        .bind(&actor_url)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            "DELETE FROM event_queue
             WHERE instr(payload, ?1) > 0
               AND event_type != 'UserDeleted'
               AND status IN ('done', 'dead_lettered')",
        )
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            "UPDATE wrap_up_records
             SET report_json = json_set(report_json, '$.most_active_user', NULL)
             WHERE user_id IS NULL AND report_json IS NOT NULL
               AND instr(report_json, ?) > 0",
        )
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        for table in ["ap_followers", "ap_following"] {
            sqlx::query(&format!("DELETE FROM {table} WHERE remote_actor_url = ?"))
                .bind(&actor_url)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }

        for (table, column) in PURGED_USER_TABLES {
            let extra = if *table == "reviews" {
                " AND remote_actor_url IS NULL"
            } else {
                ""
            };
            sqlx::query(&format!("DELETE FROM {table} WHERE {column} = ?{extra}"))
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn forget_images(&self, user: &UserId) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE users SET avatar_path = NULL, banner_path = NULL
             WHERE id = ? AND purged_at IS NOT NULL",
        )
        .bind(user.value().to_string())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn purge_expired_tombstones(
        &self,
        cutoff: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let cutoff = cutoff.format(TIMESTAMP_FORMAT).to_string();
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let expiring: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM users u
             WHERE purged_at IS NOT NULL AND purged_at < ?
               AND NOT EXISTS (
                   SELECT 1 FROM event_queue q
                   WHERE q.status NOT IN ('done', 'dead_lettered')
                     AND instr(q.payload, u.id) > 0)",
        )
        .bind(&cutoff)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        for id in &expiring {
            sqlx::query("DELETE FROM event_queue WHERE instr(payload, ?) > 0")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            for table in ["ap_followers", "ap_following"] {
                sqlx::query(&format!("DELETE FROM {table} WHERE local_user_id = ?"))
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx_error)?;
            }
            for table in RETAINED_USER_TABLES {
                sqlx::query(&format!("DELETE FROM {table} WHERE user_id = ?"))
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx_error)?;
            }
            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(expiring.len() as u64)
    }
}
