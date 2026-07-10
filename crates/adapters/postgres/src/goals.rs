use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{Goal, GoalType},
    ports::{GoalCommand, GoalQuery},
    value_objects::{GoalId, UserId},
};
use sqlx::{PgPool, Row};

use adapter_common::{datetime_to_str, parse_datetime, parse_uuid};

pub struct PostgresGoalRepository {
    pool: PgPool,
}

impl PostgresGoalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl GoalCommand for PostgresGoalRepository {
    async fn save(&self, goal: &Goal) -> Result<(), DomainError> {
        let id = goal.id().value().to_string();
        let user_id = goal.user_id().value().to_string();
        let year = goal.year() as i64;
        let target = goal.target_count() as i64;
        let goal_type = goal.goal_type().as_str();
        let created_at = datetime_to_str(goal.created_at());

        sqlx::query(
            "INSERT INTO goals (id, user_id, year, target_count, goal_type, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6::timestamptz)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(year)
        .bind(target)
        .bind(goal_type)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, goal: &Goal) -> Result<(), DomainError> {
        let id = goal.id().value().to_string();
        let target = goal.target_count() as i64;

        let result = sqlx::query("UPDATE goals SET target_count = $1 WHERE id = $2")
            .bind(target)
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Goal not found".into()));
        }
        Ok(())
    }

    async fn delete(&self, id: &GoalId, user_id: &UserId) -> Result<(), DomainError> {
        let gid = id.value().to_string();
        let uid = user_id.value().to_string();

        let result = sqlx::query("DELETE FROM goals WHERE id = $1 AND user_id = $2")
            .bind(&gid)
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Goal not found".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl GoalQuery for PostgresGoalRepository {
    async fn find_by_user_and_year(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<Goal>, DomainError> {
        let uid = user_id.value().to_string();
        let y = year as i64;

        let row = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, \
             to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM goals WHERE user_id = $1 AND year = $2",
        )
        .bind(&uid)
        .bind(y)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        row.map(|r| row_to_goal(&r)).transpose()
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, \
             to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at \
             FROM goals WHERE user_id = $1 ORDER BY year DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        rows.iter().map(row_to_goal).collect()
    }
}

pub(crate) async fn count_reviews_in_year(
    pool: &PgPool,
    user_id: &UserId,
    year: u16,
) -> Result<u32, DomainError> {
    let uid = user_id.value().to_string();
    let start = format!("{year}-01-01 00:00:00");
    let end = format!("{}-01-01 00:00:00", year + 1);

    let count: i64 = sqlx::query(
        "SELECT COUNT(*) FROM reviews \
         WHERE user_id = $1 \
         AND watched_at >= $2::timestamptz \
         AND watched_at < $3::timestamptz \
         AND remote_actor_url IS NULL",
    )
    .bind(&uid)
    .bind(&start)
    .bind(&end)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    })?
    .try_get(0)
    .unwrap_or(0);

    Ok(count as u32)
}

pub(crate) fn row_to_goal(r: &sqlx::postgres::PgRow) -> Result<Goal, DomainError> {
    let id_str: String = r
        .try_get("id")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read goal id: {e}")))?;
    let user_id_str: String = r
        .try_get("user_id")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read user_id: {e}")))?;
    let year: i64 = r
        .try_get("year")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read year: {e}")))?;
    let target: i64 = r.try_get("target_count").map_err(|e| {
        DomainError::InfrastructureError(format!("Failed to read target_count: {e}"))
    })?;
    let goal_type_str: String = r
        .try_get("goal_type")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read goal_type: {e}")))?;
    let created_at_str: String = r
        .try_get("created_at")
        .map_err(|e| DomainError::InfrastructureError(format!("Failed to read created_at: {e}")))?;

    let id = GoalId::from_uuid(parse_uuid(&id_str)?);
    let user_id = UserId::from_uuid(parse_uuid(&user_id_str)?);
    let goal_type: GoalType = goal_type_str.parse()?;
    let created_at = parse_datetime(&created_at_str)?;

    Ok(Goal::from_persistence(
        id,
        user_id,
        year as u16,
        target as u32,
        goal_type,
        created_at,
    ))
}
