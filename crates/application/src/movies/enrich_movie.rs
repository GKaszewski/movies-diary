use std::collections::HashMap;

use domain::{
    errors::DomainError,
    models::{CastMember, CrewMember, ExternalPersonId, IndexableDocument, Person, PersonId},
};

use crate::movies::{commands::EnrichMovieCommand, deps::EnrichMovieDeps};

pub async fn execute(deps: &EnrichMovieDeps, cmd: EnrichMovieCommand) -> Result<(), DomainError> {
    let persons = extract_persons(&cmd.profile.cast, &cmd.profile.crew);
    if !persons.is_empty() {
        deps.person_command.upsert_batch(&persons).await?;
    }

    let Some(movie) = deps.movie_query.get_movie_by_id(&cmd.movie_id).await? else {
        tracing::warn!(movie_id = %cmd.movie_id.value(), "enrich_movie: movie not found, skipping");
        return Ok(());
    };

    deps.search_command
        .index(IndexableDocument::Movie {
            id: cmd.movie_id.clone(),
            movie: Box::new(movie),
            profile: Some(Box::new(cmd.profile.clone())),
        })
        .await?;

    for person in &persons {
        deps.search_command
            .index(IndexableDocument::Person {
                id: person.id().clone(),
                person: Box::new(person.clone()),
            })
            .await?;
    }

    deps.movie_profile.upsert(&cmd.profile).await?;

    tracing::info!(
        movie_id = %cmd.movie_id.value(),
        persons = persons.len(),
        "enrich_movie: search index updated and profile stored"
    );
    Ok(())
}

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
