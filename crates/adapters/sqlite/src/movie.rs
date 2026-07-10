use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        Movie, MovieFilter, MovieSummary,
        collections::{PageParams, Paginated},
    },
    ports::{MovieCommand, MovieQuery},
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear},
};
use sqlx::SqlitePool;

use crate::models::{MovieRow, MovieSummaryRow};

pub struct SqliteMovieRepository {
    pool: SqlitePool,
}

impl SqliteMovieRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl MovieCommand for SqliteMovieRepository {
    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, ?, ?)
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
        .map_err(adapter_common::map_sqlx_error)?;

        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query("DELETE FROM movies WHERE id = ?")
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(adapter_common::map_sqlx_error)?;
        Ok(())
    }
}

#[async_trait]
impl MovieQuery for SqliteMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let id = external_metadata_id.value();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let t = title.value();
        let y = year.value() as i64;
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE title = ? AND release_year = ?",
        )
        .bind(t)
        .bind(y)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(MovieRow::into_domain)
        .collect()
    }

    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        if ids.is_empty() {
            return Ok(Default::default());
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT external_metadata_id FROM movies WHERE external_metadata_id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query_scalar::<_, String>(&sql);
        for id in ids {
            q = q.bind(id.value().to_string());
        }
        let rows = q.fetch_all(&self.pool).await.map_err(adapter_common::map_sqlx_error)?;
        Ok(rows.into_iter().collect())
    }

    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        if pairs.is_empty() {
            return Ok(Default::default());
        }
        let conditions: Vec<String> = pairs
            .iter()
            .map(|_| "(title = ? AND release_year = ?)".to_string())
            .collect();
        let sql = format!(
            "SELECT DISTINCT title, release_year FROM movies WHERE {}",
            conditions.join(" OR ")
        );
        use sqlx::Row;
        let mut q = sqlx::query(&sql);
        for (t, y) in pairs {
            q = q.bind(t.value().to_string()).bind(y.value() as i64);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(adapter_common::map_sqlx_error)?;
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
               GROUP_CONCAT(g.name) AS genres \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             LEFT JOIN movie_genres g ON g.movie_id = m.id \
             WHERE (? IS NULL OR LOWER(m.title) LIKE ?) \
               AND (? IS NULL OR p.original_language = ?) \
               AND (? IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER(?))) \
             GROUP BY m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path, \
                      p.overview, p.runtime_minutes, p.original_language, p.collection_name \
             ORDER BY m.title ASC \
             LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(language)
        .bind(language)
        .bind(genre)
        .bind(genre)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        let total: i64 = sqlx::query(
            "SELECT COUNT(DISTINCT m.id) \
             FROM movies m \
             LEFT JOIN movie_profiles p ON p.movie_id = m.id \
             WHERE (? IS NULL OR LOWER(m.title) LIKE ?) \
               AND (? IS NULL OR p.original_language = ?) \
               AND (? IS NULL OR m.id IN (SELECT movie_id FROM movie_genres WHERE LOWER(name) = LOWER(?)))",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(language)
        .bind(language)
        .bind(genre)
        .bind(genre)
        .fetch_one(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
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

    async fn list_movies_with_external_id(&self) -> Result<Vec<Movie>, DomainError> {
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?
        .into_iter()
        .map(MovieRow::into_domain)
        .collect()
    }
}
