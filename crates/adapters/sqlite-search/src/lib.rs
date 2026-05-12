use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        EntityType, IndexableDocument, MovieSearchHit, PersonSearchHit,
        SearchQuery, SearchResults,
        collections::Paginated,
    },
    models::PersonId,
    value_objects::MovieId,
    ports::{SearchCommand, SearchPort},
};
use sqlx::SqlitePool;

pub struct SqliteSearchAdapter {
    pool: SqlitePool,
}

impl SqliteSearchAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn create_search_adapter(pool: SqlitePool) -> (Arc<dyn SearchCommand>, Arc<dyn SearchPort>) {
    let adapter = Arc::new(SqliteSearchAdapter::new(pool));
    (Arc::clone(&adapter) as Arc<dyn SearchCommand>, adapter as Arc<dyn SearchPort>)
}

fn map_err(e: sqlx::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl SearchCommand for SqliteSearchAdapter {
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError> {
        match doc {
            IndexableDocument::Movie { id, movie, profile } => {
                let movie_id = id.value().to_string();
                let title = movie.title().value().to_string();
                let director = movie.director().unwrap_or("").to_string();
                let release_year = movie.release_year().value() as i64;
                let (overview, genres, keywords, cast_names, crew_names, language) =
                    match profile.as_deref() {
                        Some(p) => (
                            p.overview.clone().unwrap_or_default(),
                            p.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(" "),
                            p.keywords.iter().map(|k| k.name.as_str()).collect::<Vec<_>>().join(" "),
                            p.cast.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(" "),
                            p.crew.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(" "),
                            p.original_language.clone().unwrap_or_default(),
                        ),
                        None => (String::new(), String::new(), String::new(), String::new(), String::new(), String::new()),
                    };

                sqlx::query(
                    "DELETE FROM movies_fts WHERE rowid = (SELECT rowid FROM movies_fts WHERE movie_id = ? LIMIT 1)",
                )
                .bind(&movie_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

                sqlx::query(
                    "INSERT INTO movies_fts(movie_id, title, director, overview, genres, keywords, cast_names, crew_names, release_year, language)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&movie_id)
                .bind(&title)
                .bind(&director)
                .bind(&overview)
                .bind(&genres)
                .bind(&keywords)
                .bind(&cast_names)
                .bind(&crew_names)
                .bind(release_year)
                .bind(&language)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

                Ok(())
            }

            IndexableDocument::Person { id, person } => {
                let person_id = id.value().to_string();

                sqlx::query(
                    "DELETE FROM people_fts WHERE rowid = (SELECT rowid FROM people_fts WHERE person_id = ? LIMIT 1)",
                )
                .bind(&person_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;

                sqlx::query(
                    "INSERT INTO people_fts(person_id, name, known_for_department) VALUES (?, ?, ?)",
                )
                .bind(&person_id)
                .bind(person.name())
                .bind(person.known_for_department())
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
                sqlx::query(
                    "DELETE FROM movies_fts WHERE rowid = (SELECT rowid FROM movies_fts WHERE movie_id = ? LIMIT 1)",
                )
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            }
            EntityType::Person => {
                sqlx::query(
                    "DELETE FROM people_fts WHERE rowid = (SELECT rowid FROM people_fts WHERE person_id = ? LIMIT 1)",
                )
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
impl SearchPort for SqliteSearchAdapter {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, DomainError> {
        let movies = self.search_movies(query).await?;
        let people = self.search_people(query).await?;
        Ok(SearchResults { movies, people })
    }
}

impl SqliteSearchAdapter {
    async fn search_movies(&self, query: &SearchQuery) -> Result<Paginated<MovieSearchHit>, DomainError> {
        let limit = query.page.limit as i64;
        let offset = query.page.offset as i64;

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            title: String,
            release_year: Option<i64>,
            director: Option<String>,
            poster_path: Option<String>,
            genres: Option<String>,
        }

        let total: u64 = if let Some(text) = &query.text {
            let fts_query = format!("{}*", text.replace(['"', '*'], ""));
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT m.id)
                 FROM movies_fts fts
                 JOIN movies m ON m.id = fts.movie_id
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE movies_fts MATCH ?
                   AND (? IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = ?))
                   AND (? IS NULL OR m.release_year = ?)",
            )
            .bind(&fts_query)
            .bind(&query.filters.genre)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i64))
            .bind(query.filters.year.map(|y| y as i64))
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            count as u64
        } else {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT m.id)
                 FROM movies m
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE (? IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = ?))
                   AND (? IS NULL OR m.release_year = ?)",
            )
            .bind(&query.filters.genre)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i64))
            .bind(query.filters.year.map(|y| y as i64))
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            count as u64
        };

        let rows: Vec<Row> = if let Some(text) = &query.text {
            let fts_query = format!("{}*", text.replace(['"', '*'], ""));
            sqlx::query_as::<_, Row>(
                "SELECT m.id, m.title, m.release_year, m.director, m.poster_path,
                        GROUP_CONCAT(DISTINCT mg.name) AS genres
                 FROM movies_fts fts
                 JOIN movies m ON m.id = fts.movie_id
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE movies_fts MATCH ?
                   AND (? IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = ?))
                   AND (? IS NULL OR m.release_year = ?)
                 GROUP BY m.id
                 ORDER BY rank
                 LIMIT ? OFFSET ?",
            )
            .bind(&fts_query)
            .bind(&query.filters.genre)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i64))
            .bind(query.filters.year.map(|y| y as i64))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        } else {
            sqlx::query_as::<_, Row>(
                "SELECT m.id, m.title, m.release_year, m.director, m.poster_path,
                        GROUP_CONCAT(DISTINCT mg.name) AS genres
                 FROM movies m
                 LEFT JOIN movie_genres mg ON mg.movie_id = m.id
                 WHERE (? IS NULL OR EXISTS (SELECT 1 FROM movie_genres WHERE movie_id = m.id AND name = ?))
                   AND (? IS NULL OR m.release_year = ?)
                 GROUP BY m.id
                 ORDER BY m.title
                 LIMIT ? OFFSET ?",
            )
            .bind(&query.filters.genre)
            .bind(&query.filters.genre)
            .bind(query.filters.year.map(|y| y as i64))
            .bind(query.filters.year.map(|y| y as i64))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        };
        let items = rows.into_iter().map(|r| MovieSearchHit {
            movie_id: MovieId::from_uuid(uuid::Uuid::parse_str(&r.id).unwrap_or_default()),
            title: r.title,
            release_year: r.release_year.map(|y| y as u16),
            director: r.director,
            poster_path: r.poster_path,
            genres: r.genres
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        }).collect::<Vec<_>>();

        Ok(Paginated { items, total_count: total, limit: query.page.limit, offset: query.page.offset })
    }

    async fn search_people(&self, query: &SearchQuery) -> Result<Paginated<PersonSearchHit>, DomainError> {
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
        let fts_query = format!("{}*", text.replace(['"', '*'], ""));

        let total: u64 = {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM people_fts WHERE people_fts MATCH ?",
            )
            .bind(&fts_query)
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
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT fts.person_id, p.name, p.known_for_department, p.profile_path
             FROM people_fts fts
             JOIN persons p ON p.id = fts.person_id
             WHERE people_fts MATCH ?
             ORDER BY rank
             LIMIT ? OFFSET ?",
        )
        .bind(&fts_query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let tmdb_id: Option<i64> = sqlx::query_scalar(
                "SELECT tmdb_person_id FROM persons WHERE id = ?",
            )
            .bind(&row.person_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?
            .flatten();

            let known_for_titles = if let Some(tid) = tmdb_id {
                sqlx::query_scalar::<_, String>(
                    "SELECT m.title FROM movie_cast mc
                     JOIN movies m ON m.id = mc.movie_id
                     WHERE mc.tmdb_person_id = ?
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
                    uuid::Uuid::parse_str(&row.person_id).unwrap_or_default()
                ),
                name: row.name,
                known_for_department: row.known_for_department,
                profile_path: row.profile_path,
                known_for_titles,
            });
        }

        Ok(Paginated { items, total_count: total, limit: query.page.limit, offset: query.page.offset })
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
