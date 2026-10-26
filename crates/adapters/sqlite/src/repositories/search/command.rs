use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{EntityType, IndexableDocument},
    ports::SearchCommand,
};

use super::SqliteSearchAdapter;

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
                            p.original_language.clone().unwrap_or_default(),
                        ),
                        None => (
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                        ),
                    };

                sqlx::query(
                    "DELETE FROM movies_fts WHERE rowid = (SELECT rowid FROM movies_fts WHERE movie_id = ? LIMIT 1)",
                )
                .bind(&movie_id)
                .execute(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error)?;

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
                .map_err(crate::shared::map_sqlx_error)?;

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
                .map_err(crate::shared::map_sqlx_error)?;

                sqlx::query(
                    "INSERT INTO people_fts(person_id, name, known_for_department) VALUES (?, ?, ?)",
                )
                .bind(&person_id)
                .bind(person.name())
                .bind(person.known_for_department())
                .execute(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error)?;

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
                .map_err(crate::shared::map_sqlx_error)?;
            }
            EntityType::Person => {
                sqlx::query(
                    "DELETE FROM people_fts WHERE rowid = (SELECT rowid FROM people_fts WHERE person_id = ? LIMIT 1)",
                )
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error)?;
            }
        }
        Ok(())
    }
}
