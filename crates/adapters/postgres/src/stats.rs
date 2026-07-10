use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DirectorStat, MonthlyRating, UserStats, UserTrends},
    ports::StatsRepository,
    value_objects::UserId,
};
use sqlx::PgPool;

use crate::models::{
    DirectorCountRow, GenreCountRow, MonthlyRatingRow, RatingDistRow, UserTotalsRow,
    WatchMediumCountRow,
};
use adapter_common::format_year_month;

pub struct PostgresStatsRepository {
    pool: PgPool,
}

impl PostgresStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn fetch_user_totals(&self, user_id: &str) -> Result<UserTotalsRow, DomainError> {
        sqlx::query_as::<_, UserTotalsRow>(
            r#"SELECT COUNT(DISTINCT movie_id) AS total,
                      AVG(rating::float) AS avg_rating
               FROM reviews WHERE user_id = $1"#,
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
        sqlx::query_scalar::<_, String>(
            "SELECT m.director
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1 AND m.director IS NOT NULL
             GROUP BY m.director
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)
    }

    async fn fetch_user_most_active_month(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, DomainError> {
        sqlx::query_scalar::<_, String>(
            "SELECT to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') AS month
             FROM reviews
             WHERE user_id = $1
             GROUP BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM')
             ORDER BY COUNT(*) DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)
    }
}

#[async_trait]
impl StatsRepository for PostgresStatsRepository {
    async fn get_user_stats(&self, user_id: &UserId) -> Result<UserStats, DomainError> {
        let uid = user_id.value().to_string();

        let (totals, fav_director, most_active) = tokio::try_join!(
            self.fetch_user_totals(&uid),
            self.fetch_user_favorite_director(&uid),
            self.fetch_user_most_active_month(&uid)
        )?;

        let most_active_month = most_active.map(|ym| format_year_month(&ym));

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

        let (rating_rows, director_rows, genre_rows, rating_dist_rows, medium_rows) =
            tokio::try_join!(
                sqlx::query_as::<_, MonthlyRatingRow>(
                    "SELECT to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') AS month,
                        AVG(rating::float) AS avg_rating,
                        COUNT(*) AS count
                 FROM reviews
                 WHERE user_id = $1 AND watched_at >= NOW() - INTERVAL '12 months'
                 GROUP BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM')
                 ORDER BY to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM') ASC"
                )
                .bind(&uid)
                .fetch_all(&self.pool),
                sqlx::query_as::<_, DirectorCountRow>(
                    "SELECT m.director AS director, COUNT(*) AS count
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND m.director IS NOT NULL
                 GROUP BY m.director
                 ORDER BY COUNT(*) DESC
                 LIMIT 5"
                )
                .bind(&uid)
                .fetch_all(&self.pool),
                sqlx::query_as::<_, GenreCountRow>(
                    "SELECT mg.name AS genre, COUNT(*) AS count
                 FROM reviews r
                 INNER JOIN movie_genres mg ON mg.movie_id = r.movie_id
                 WHERE r.user_id = $1
                 GROUP BY mg.name
                 ORDER BY COUNT(*) DESC
                 LIMIT 5"
                )
                .bind(&uid)
                .fetch_all(&self.pool),
                sqlx::query_as::<_, RatingDistRow>(
                    "SELECT rating, COUNT(*) AS count
                 FROM reviews
                 WHERE user_id = $1
                 GROUP BY rating
                 ORDER BY rating ASC"
                )
                .bind(&uid)
                .fetch_all(&self.pool),
                sqlx::query_as::<_, WatchMediumCountRow>(
                    "SELECT watch_medium, COUNT(*) AS count
                 FROM reviews
                 WHERE user_id = $1 AND watch_medium IS NOT NULL
                 GROUP BY watch_medium
                 ORDER BY COUNT(*) DESC"
                )
                .bind(&uid)
                .fetch_all(&self.pool)
            )
            .map_err(adapter_common::map_sqlx_error)?;

        let max_director_count = director_rows.iter().map(|d| d.count).max().unwrap_or(1);

        let monthly_ratings = rating_rows
            .into_iter()
            .map(|r| MonthlyRating {
                month_label: format_year_month(&r.month),
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

        let top_genres = genre_rows
            .into_iter()
            .map(|g| domain::models::stats::GenreStat {
                genre: g.genre,
                count: g.count,
            })
            .collect();

        let rating_distribution = {
            let mut dist = [0i64; 5];
            for r in &rating_dist_rows {
                let idx = (r.rating as usize).saturating_sub(1).min(4);
                dist[idx] = r.count;
            }
            dist
        };

        let watch_medium_distribution = medium_rows
            .into_iter()
            .map(|m| domain::models::stats::WatchMediumStat {
                medium: m.watch_medium,
                count: m.count,
            })
            .collect();

        Ok(UserTrends {
            monthly_ratings,
            top_directors,
            max_director_count,
            top_genres,
            rating_distribution,
            watch_medium_distribution,
        })
    }
}
