use std::collections::HashMap;

use domain::{
    errors::DomainError,
    models::{CastMember, CrewMember, ExternalPersonId, IndexableDocument, Person, PersonId},
};

use crate::movies::{commands::EnrichMovieCommand, deps::EnrichMovieDeps};

pub async fn execute(deps: &EnrichMovieDeps, cmd: EnrichMovieCommand) -> Result<(), DomainError> {
    // 1. Persist the enriched profile (also handles movie_cast, movie_crew, genres, keywords)
    deps.movie_profile.upsert(&cmd.profile).await?;

    // 2. Upsert persons extracted from cast + crew (no reads — only upsert)
    let persons = extract_persons(&cmd.profile.cast, &cmd.profile.crew);
    if !persons.is_empty() {
        deps.person_command.upsert_batch(&persons).await?;
    }

    // 3. Fetch the movie for the search index document
    let Some(movie) = deps.movie.get_movie_by_id(&cmd.movie_id).await? else {
        tracing::warn!(movie_id = %cmd.movie_id.value(), "enrich_movie: movie not found after profile upsert");
        return Ok(());
    };

    // 4. Index the movie in search
    deps.search_command
        .index(IndexableDocument::Movie {
            id: cmd.movie_id.clone(),
            movie: Box::new(movie),
            profile: Some(Box::new(cmd.profile.clone())),
        })
        .await?;

    // 5. Index each unique person in search (no reads — persons built from in-memory data)
    for person in &persons {
        deps.search_command
            .index(IndexableDocument::Person {
                id: person.id().clone(),
                person: Box::new(person.clone()),
            })
            .await?;
    }

    tracing::info!(
        movie_id = %cmd.movie_id.value(),
        persons = persons.len(),
        "enrich_movie: profile stored and search index updated"
    );
    Ok(())
}

/// Build unique Person values from cast and crew.
/// Uses deterministic UUIDv5 so the same tmdb_person_id always maps to the same PersonId.
/// No DB reads — persons are built entirely from in-memory TMDb data.
fn extract_persons(cast: &[CastMember], crew: &[CrewMember]) -> Vec<Person> {
    let mut seen: HashMap<u64, Person> = HashMap::new();

    for member in cast {
        seen.entry(member.tmdb_person_id).or_insert_with(|| {
            let ext = ExternalPersonId::new(format!("tmdb:{}", member.tmdb_person_id));
            Person::basic(
                PersonId::from_external(&ext),
                ext,
                member.name.clone(),
                Some("Acting".to_string()),
                member.profile_path.clone(),
            )
        });
    }

    for member in crew {
        seen.entry(member.tmdb_person_id).or_insert_with(|| {
            let ext = ExternalPersonId::new(format!("tmdb:{}", member.tmdb_person_id));
            Person::basic(
                PersonId::from_external(&ext),
                ext,
                member.name.clone(),
                Some(member.department.clone()),
                member.profile_path.clone(),
            )
        });
    }

    seen.into_values().collect()
}

#[cfg(test)]
#[path = "tests/enrich_movie.rs"]
mod tests;
