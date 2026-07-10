use api_types::search::PersonDto;
use domain::models::person::Person;

pub fn person_to_dto(p: &Person) -> PersonDto {
    PersonDto {
        id: p.id().value(),
        external_id: p.external_id().value().to_string(),
        name: p.name().to_string(),
        known_for_department: p.known_for_department().map(str::to_string),
        profile_path: p.profile_path().map(str::to_string),
        biography: p.biography().map(str::to_string),
        birthday: p.birthday().map(|d| d.to_string()),
        deathday: p.deathday().map(|d| d.to_string()),
        place_of_birth: p.place_of_birth().map(str::to_string),
        also_known_as: p.also_known_as().to_vec(),
        homepage: p.homepage().map(str::to_string),
        imdb_url: p
            .imdb_id()
            .map(|id| format!("https://www.imdb.com/name/{id}")),
        enriched: p.enriched_at().is_some(),
    }
}
