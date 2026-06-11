use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::{CastMember, CrewMember, ExternalPersonId, MovieProfile, PersonId},
    ports::MovieProfileRepository,
    value_objects::MovieId,
};
use uuid::Uuid;

pub struct GetMovieProfileQuery {
    pub movie_id: Uuid,
}

pub struct CastMemberWithId {
    pub person_id: PersonId,
    pub tmdb_person_id: u64,
    pub name: String,
    pub character: String,
    pub billing_order: u32,
    pub profile_path: Option<String>,
}

pub struct CrewMemberWithId {
    pub person_id: PersonId,
    pub tmdb_person_id: u64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

pub struct MovieProfileResult {
    pub profile: MovieProfile,
    pub cast: Vec<CastMemberWithId>,
    pub crew: Vec<CrewMemberWithId>,
}

fn resolve_cast(member: &CastMember) -> CastMemberWithId {
    let ext = ExternalPersonId::new(format!("tmdb:{}", member.tmdb_person_id));
    CastMemberWithId {
        person_id: PersonId::from_external(&ext),
        tmdb_person_id: member.tmdb_person_id,
        name: member.name.clone(),
        character: member.character.clone(),
        billing_order: member.billing_order,
        profile_path: member.profile_path.clone(),
    }
}

fn resolve_crew(member: &CrewMember) -> CrewMemberWithId {
    let ext = ExternalPersonId::new(format!("tmdb:{}", member.tmdb_person_id));
    CrewMemberWithId {
        person_id: PersonId::from_external(&ext),
        tmdb_person_id: member.tmdb_person_id,
        name: member.name.clone(),
        job: member.job.clone(),
        department: member.department.clone(),
        profile_path: member.profile_path.clone(),
    }
}

pub async fn execute(
    movie_profile: Arc<dyn MovieProfileRepository>,
    query: GetMovieProfileQuery,
) -> Result<Option<MovieProfileResult>, DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);
    let profile = movie_profile.get_by_movie_id(&movie_id).await?;

    Ok(profile.map(|p| {
        let cast = p.cast.iter().map(resolve_cast).collect();
        let crew = p.crew.iter().map(resolve_crew).collect();
        MovieProfileResult {
            profile: p,
            cast,
            crew,
        }
    }))
}

#[cfg(test)]
#[path = "tests/get_movie_profile.rs"]
mod tests;
