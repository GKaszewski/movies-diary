use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        Movie, MovieFilter, MovieSummary,
        collections::{PageParams, Paginated},
    },
    ports::MovieRepository,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear},
};
use sqlx::PgPool;

use crate::models::{MovieRow, MovieSummaryRow};

pub struct PostgresMovieRepository {
    pool: PgPool,
}

impl PostgresMovieRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl MovieRepository for PostgresMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let id = external_metadata_id.value();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let title = title.value();
        let year = year.value() as i64;
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE title = $1 AND release_year = $2",
        )
        .bind(title)
        .bind(year)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(MovieRow::into_domain)
        .collect()
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = excluded.external_metadata_id,
                 title                = excluded.title,
                 release_year         = excluded.release_year,
                 director             = excluded.director,
                 poster_path          = excluded.poster_path",
        )
        .bind(&id)
        .bind(&external_metadata_id)
        .bind(title)
        .bind(release_year)
        .bind(director)
        .bind(&poster_path)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query("DELETE FROM movies WHERE id = $1")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        if ids.is_empty() {
            return Ok(Default::default());
        }
        let vals: Vec<String> = ids.iter().map(|id| id.value().to_string()).collect();
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT external_metadata_id FROM movies WHERE external_metadata_id = ANY($1)",
        )
        .bind(&vals)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        if pairs.is_empty() {
            return Ok(Default::default());
        }
        let titles: Vec<&str> = pairs.iter().map(|(t, _)| t.value()).collect();
        let years: Vec<i64> = pairs.iter().map(|(_, y)| y.value() as i64).collect();
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT DISTINCT m.title, m.release_year FROM movies m \
             INNER JOIN unnest($1::text[], $2::bigint[]) AS p(title, release_year) \
             ON m.title = p.title AND m.release_year = p.release_year",
        )
        .bind(&titles)
        .bind(&years)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let t: String = r.get("title");
                let y: i64 = r.get("release_year");
                (t, y as u16)
            })
            .collect())
    }

    async fn list_movies(
        &self,
        page: &PageParams,
        filter: &MovieFilter,
    ) -> Result<Paginated<MovieSummary>, DomainError> {
        use sqlx::Row;
        let limit = page.limit as i64;
        let offset = page.offset as i64;
        let pattern = filter
            .search
            .as_deref()
            .map(|s| format!("%{}%", s.to_lowercase()));
        let genre = filter.genre.as_deref();
        let language = filter.language.as_deref();

        let rows: Vec<MovieSummaryRow> = sqlx::query_as(
            "SELECT \
               m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path, \
               p.overview, p.runtime_minutes, p.original_language, p.collection_name, \
               array_agg(g.name) FILTER (WHERE g.name IS NOT NULL) AS genres \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             LEFT JOIN movie_genres g ON g.movie_id = m.id \
             WHERE ($1::text IS NULL OR LOWER(m.title) LIKE $1) \
               AND ($2::text IS NULL OR p.original_language = $2) \
               AND ($3::text IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER($3))) \
             GROUP BY m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path, \
                      p.overview, p.runtime_minutes, p.original_language, p.collection_name \
             ORDER BY m.title ASC \
             LIMIT $4 OFFSET $5",
        )
        .bind(&pattern)
        .bind(language)
        .bind(genre)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let total: i64 = sqlx::query(
            "SELECT COUNT(DISTINCT m.id) \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             WHERE ($1::text IS NULL OR LOWER(m.title) LIKE $1) \
               AND ($2::text IS NULL OR p.original_language = $2) \
               AND ($3::text IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER($3)))",
        )
        .bind(&pattern)
        .bind(language)
        .bind(genre)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?
        .try_get(0)
        .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|r| r.into_domain())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }
}
