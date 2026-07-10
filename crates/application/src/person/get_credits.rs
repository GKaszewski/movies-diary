use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{PersonCredits, PersonId},
};

use super::{deps::GetPersonDeps, should_enrich};

pub async fn execute(deps: &GetPersonDeps, id: PersonId) -> Result<PersonCredits, DomainError> {
    let credits = deps.person_query.get_credits(&id).await?;
    if should_enrich(&credits.person) {
        let _ = deps
            .event_publisher
            .publish(&DomainEvent::PersonEnrichmentRequested {
                person_id: id,
                external_person_id: credits.person.external_id().clone(),
            })
            .await;
    }
    Ok(credits)
}

#[cfg(test)]
#[path = "tests/get_credits.rs"]
mod tests;
