use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DirectorStat, MonthlyRating, UserStats, UserTrends},
    ports::StatsRepository,
    value_objects::UserId,
};
use sqlx::SqlitePool;

use crate::models::{DirectorCountRow, MonthlyRatingRow, UserTotalsRow};

pub struct SqliteStatsRepository {
    pool: SqlitePool,
}

impl SqliteStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn fetch_user_totals(&self, user_id: &str) -> Result<UserTotalsRow, DomainError> {
        sqlx::query_as::<_, UserTotalsRow>(
            "SELECT COUNT(DISTINCT movie_id) AS total,
                    AVG(CAST(rating AS REAL)) AS avg_rating
             FROM reviews WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)
    }

    async fn fetch_user_favorite_director(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        let row: Option<String> = sqlx::query_scalar(
            "SELECT m.director
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ? AND m.director IS NOT NULL
             GROUP BY m.director
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        Ok(row)
    }

    async fn fetch_user_most_active_month(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        let row: Option<String> = sqlx::query_scalar(
            "SELECT strftime('%Y-%m', watched_at)
             FROM reviews
             WHERE user_id = ?
             GROUP BY strftime('%Y-%m', watched_at)
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        Ok(row)
    }
}

#[async_trait]
impl StatsRepository for SqliteStatsRepository {
    async fn get_user_stats(&self, user_id: &UserId) -> Result<UserStats, DomainError> {
        let uid = user_id.value().to_string();

        let (totals, fav_director, most_active) = tokio::try_join!(
            self.fetch_user_totals(&uid),
            self.fetch_user_favorite_director(&uid),
            self.fetch_user_most_active_month(&uid)
        )?;

        let most_active_month = most_active.map(|ym| adapter_common::format_year_month(&ym));

        Ok(UserStats {
            total_movies: totals.total,
            avg_rating: totals.avg_rating,
            favorite_director: fav_director,
            most_active_month,
        })
    }

    async fn count_reviews_in_year(&self, user_id: &UserId, year: u16) -> Result<u32, DomainError> {
        crate::goals::count_reviews_in_year(&self.pool, user_id, year).await
    }

    async fn get_user_trends(&self, user_id: &UserId) -> Result<UserTrends, DomainError> {
        let uid = user_id.value().to_string();

        let (rating_rows, director_rows) = tokio::try_join!(
            sqlx::query_as::<_, MonthlyRatingRow>(
                "SELECT strftime('%Y-%m', watched_at) AS month,
                        AVG(CAST(rating AS REAL)) AS avg_rating,
                        COUNT(*) AS count
                 FROM reviews
                 WHERE user_id = ? AND watched_at >= datetime('now', '-12 months')
                 GROUP BY month
                 ORDER BY month ASC",
            )
            .bind(&uid)
            .fetch_all(&self.pool),
            sqlx::query_as::<_, DirectorCountRow>(
                "SELECT m.director,
                        COUNT(*) AS count
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ? AND m.director IS NOT NULL
                 GROUP BY m.director
                 ORDER BY COUNT(*) DESC
                 LIMIT 5",
            )
            .bind(&uid)
            .fetch_all(&self.pool)
        )
        .map_err(adapter_common::map_sqlx_error)?;

        let max_director_count = director_rows.iter().map(|d| d.count).max().unwrap_or(1);

        let monthly_ratings = rating_rows
            .into_iter()
            .map(|r| MonthlyRating {
                month_label: adapter_common::format_year_month(&r.month),
                year_month: r.month,
                avg_rating: r.avg_rating,
                count: r.count,
            })
            .collect();

        let top_directors = director_rows
            .into_iter()
            .map(|d| DirectorStat {
                director: d.director,
                count: d.count,
            })
            .collect();

        Ok(UserTrends {
            monthly_ratings,
            top_directors,
            max_director_count,
        })
    }
}
