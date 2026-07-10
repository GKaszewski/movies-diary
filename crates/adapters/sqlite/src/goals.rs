use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{Goal, GoalType},
    ports::GoalRepository,
    value_objects::{GoalId, UserId},
};
use sqlx::{Row, SqlitePool};

pub struct SqliteGoalRepository {
    pool: SqlitePool,
}

impl SqliteGoalRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl GoalRepository for SqliteGoalRepository {
    async fn save(&self, goal: &Goal) -> Result<(), DomainError> {
        let id = goal.id().value().to_string();
        let user_id = goal.user_id().value().to_string();
        let year = goal.year() as i64;
        let target = goal.target_count() as i64;
        let goal_type = goal.goal_type().as_str();
        let created_at = goal.created_at().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "INSERT INTO goals (id, user_id, year, target_count, goal_type, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(year)
        .bind(target)
        .bind(goal_type)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn update(&self, goal: &Goal) -> Result<(), DomainError> {
        let id = goal.id().value().to_string();
        let target = goal.target_count() as i64;

        let result = sqlx::query("UPDATE goals SET target_count = ? WHERE id = ?")
            .bind(target)
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Goal not found".into()));
        }
        Ok(())
    }

    async fn delete(&self, id: &GoalId, user_id: &UserId) -> Result<(), DomainError> {
        let gid = id.value().to_string();
        let uid = user_id.value().to_string();

        let result = sqlx::query("DELETE FROM goals WHERE id = ? AND user_id = ?")
            .bind(&gid)
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Goal not found".into()));
        }
        Ok(())
    }

    async fn find_by_user_and_year(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<Goal>, DomainError> {
        let uid = user_id.value().to_string();
        let y = year as i64;

        let row = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, created_at \
             FROM goals WHERE user_id = ? AND year = ?",
        )
        .bind(&uid)
        .bind(y)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        row.map(|r| row_to_goal(&r)).transpose()
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<Goal>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, created_at \
             FROM goals WHERE user_id = ? ORDER BY year DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.iter().map(row_to_goal).collect()
    }
}

pub(crate) async fn count_reviews_in_year(
    pool: &SqlitePool,
    user_id: &UserId,
    year: u16,
) -> Result<u32, DomainError> {
    let uid = user_id.value().to_string();
    let start = format!("{year}-01-01 00:00:00");
    let end = format!("{}-01-01 00:00:00", year + 1);

    let count: i64 = sqlx::query(
        "SELECT COUNT(*) FROM reviews \
         WHERE user_id = ? AND watched_at >= ? AND watched_at < ? \
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

pub(crate) fn row_to_goal(r: &sqlx::sqlite::SqliteRow) -> Result<Goal, DomainError> {
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

    let id = GoalId::from_uuid(
        uuid::Uuid::parse_str(&id_str)
            .map_err(|e| DomainError::InfrastructureError(format!("Invalid goal UUID: {e}")))?,
    );
    let user_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&user_id_str)
            .map_err(|e| DomainError::InfrastructureError(format!("Invalid user UUID: {e}")))?,
    );
    let goal_type: GoalType = goal_type_str.parse()?;
    let created_at = chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid datetime: {e}")))?;

    Ok(Goal::from_persistence(
        id,
        user_id,
        year as u16,
        target as u32,
        goal_type,
        created_at,
    ))
}
