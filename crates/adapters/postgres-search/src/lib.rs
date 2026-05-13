use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::PersonId,
    models::{
        collections::Paginated, EntityType, IndexableDocument, MovieSearchHit, PersonSearchHit,
        SearchQuery, SearchResults,
    },
    ports::{SearchCommand, SearchPort},
    value_objects::MovieId,
};
use sqlx::PgPool;

pub struct PostgresSearchAdapter {
    pool: PgPool,
}

impl PostgresSearchAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub fn create_search_adapter(pool: PgPool) -> (Arc<dyn SearchCommand>, Arc<dyn SearchPort>) {
    let adapter = Arc::new(PostgresSearchAdapter::new(pool));
    (
        Arc::clone(&adapter) as Arc<dyn SearchCommand>,
        adapter as Arc<dyn SearchPort>,
    )
}

fn map_err(e: sqlx::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl SearchCommand for PostgresSearchAdapter {
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError> {
        match doc {
            IndexableDocument::Movie { id, movie, profile } => {
                let movie_id = id.value().to_string();
                let title = movie.title().value().to_string();
                let director = movie.director().unwrap_or("").to_string();
                let (overview, genres, keywords, cast_names, crew_names) = match profile.as_deref()
                {
                    Some(p) => (
                        p.overview.clone().unwrap_or_default(),
                        p.genres
                            .iter()
                            .map(|g| g.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        p.keywords
                            .iter()
                            .map(|k| k.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        p.cast
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        p.crew
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    None => (
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                };

                let fts_input = format!(
                    "{} {} {} {} {} {} {}",
                    title, director, overview, genres, keywords, cast_names, crew_names
                );

                sqlx::query(
                    "INSERT INTO movies_search (movie_id, fts)
                     VALUES ($1, to_tsvector('english', $2))
                     ON CONFLICT (movie_id) DO UPDATE SET fts = EXCLUDED.fts",
                )
                .bind(&movie_id)
                .bind(&fts_input)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

                Ok(())
            }

            IndexableDocument::Person { id, person } => {
                let person_id = id.value().to_string();
                let fts_input = format!(
                    "{} {}",
                    person.name(),
                    person.known_for_department().unwrap_or("")
                );

                sqlx::query(
                    "INSERT INTO people_search (person_id, fts)
                     VALUES ($1, to_tsvector('english', $2))
                     ON CONFLICT (person_id) DO UPDATE SET fts = EXCLUDED.fts",
                )
                .bind(&person_id)
                .bind(&fts_input)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

                Ok(())
            }
        }
    }

    async fn remove(&self, entity_type: EntityType, id: &str) -> Result<(), DomainError> {
        match entity_type {
            EntityType::Movie => {
                sqlx::query("DELETE FROM movies_search WHERE movie_id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
            }
            EntityType::Person => {
                sqlx::query("DELETE FROM people_search WHERE person_id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl SearchPort for PostgresSearchAdapter {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, DomainError> {
        let movies = self.search_movies(query).await?;
        let people = self.search_people(query).await?;
        Ok(SearchResults { movies, people })
    }
}

impl PostgresSearchAdapter {
    async fn search_movies(
        &self,
        query: &SearchQuery,
    ) -> Result<Paginated<MovieSearchHit>, DomainError> {
        let limit = query.page.limit as i64;
        let offset = query.page.offset as i64;

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            title: String,
            release_year: Option<i32>,
            director: Option<String>,
            poster_path: Option<String>,
            genres: Option<String>,
        }

        let total: u64 = if let Some(text) = &query.text {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT m.id)
                 FROM movies_search ms
                 JOIN movies m ON m.id = ms.movie_id
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE ms.fts @@ plainto_tsquery('english', $1)
                   AND ($2::TEXT IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = $2))
                   AND ($3::INT IS NULL OR m.release_year = $3)",
            )
            .bind(text)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i32))
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            count as u64
        } else {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT m.id) FROM movies m
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE ($1::TEXT IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = $1))
                   AND ($2::INT IS NULL OR m.release_year = $2)",
            )
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i32))
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            count as u64
        };

        let rows: Vec<Row> = if let Some(text) = &query.text {
            sqlx::query_as::<_, Row>(
                "SELECT m.id, m.title, m.release_year, m.director, m.poster_path,
                        STRING_AGG(DISTINCT mg.name, ',' ORDER BY mg.name) AS genres
                 FROM movies_search ms
                 JOIN movies m ON m.id = ms.movie_id
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE ms.fts @@ plainto_tsquery('english', $1)
                   AND ($2::TEXT IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = $2))
                   AND ($3::INT IS NULL OR m.release_year = $3)
                 GROUP BY m.id, m.title, m.release_year, m.director, m.poster_path, ms.fts
                 ORDER BY ts_rank(ms.fts, plainto_tsquery('english', $1)) DESC
                 LIMIT $4 OFFSET $5",
            )
            .bind(text)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i32))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        } else {
            sqlx::query_as::<_, Row>(
                "SELECT m.id, m.title, m.release_year, m.director, m.poster_path,
                        STRING_AGG(DISTINCT mg.name, ',' ORDER BY mg.name) AS genres
                 FROM movies m
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE ($1::TEXT IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = $1))
                   AND ($2::INT IS NULL OR m.release_year = $2)
                 GROUP BY m.id ORDER BY m.title LIMIT $3 OFFSET $4",
            )
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i32))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        };

        let items = rows
            .into_iter()
            .map(|r| MovieSearchHit {
                movie_id: MovieId::from_uuid(uuid::Uuid::parse_str(&r.id).unwrap_or_default()),
                title: r.title,
                release_year: r.release_year.map(|y| y as u16),
                director: r.director,
                poster_path: r.poster_path,
                genres: r
                    .genres
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
            })
            .collect::<Vec<_>>();

        Ok(Paginated {
            items,
            total_count: total,
            limit: query.page.limit,
            offset: query.page.offset,
        })
    }

    async fn search_people(
        &self,
        query: &SearchQuery,
    ) -> Result<Paginated<PersonSearchHit>, DomainError> {
        let Some(text) = &query.text else {
            return Ok(Paginated {
                items: vec![],
                total_count: 0,
                limit: query.page.limit,
                offset: query.page.offset,
            });
        };

        let limit = query.page.limit as i64;
        let offset = query.page.offset as i64;

        let total: u64 = {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM people_search WHERE fts @@ plainto_tsquery('english', $1)",
            )
            .bind(text)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            count as u64
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            person_id: String,
            name: String,
            known_for_department: Option<String>,
            profile_path: Option<String>,
            tmdb_person_id: Option<i64>,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT ps.person_id, p.name, p.known_for_department, p.profile_path, p.tmdb_person_id
             FROM people_search ps
             JOIN persons p ON p.id = ps.person_id
             WHERE ps.fts @@ plainto_tsquery('english', $1)
             ORDER BY ts_rank(ps.fts, plainto_tsquery('english', $1)) DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(text)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let known_for_titles = if let Some(tid) = row.tmdb_person_id {
                sqlx::query_scalar::<_, String>(
                    "SELECT m.title FROM movie_cast mc
                     JOIN movies m ON m.id = mc.movie_id
                     WHERE mc.tmdb_person_id = $1
                     ORDER BY mc.billing_order
                     LIMIT 3",
                )
                .bind(tid)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
            } else {
                vec![]
            };

            items.push(PersonSearchHit {
                person_id: PersonId::from_uuid(
                    uuid::Uuid::parse_str(&row.person_id).unwrap_or_default(),
                ),
                name: row.name,
                known_for_department: row.known_for_department,
                profile_path: row.profile_path,
                known_for_titles,
            });
        }

        Ok(Paginated {
            items,
            total_count: total,
            limit: query.page.limit,
            offset: query.page.offset,
        })
    }
}
