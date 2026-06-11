use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Person, PersonCredits, PersonId},
};

use super::deps::GetPersonDeps;

const ENRICHMENT_TTL_DAYS: i64 = 90;

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

fn should_enrich(p: &Person) -> bool {
    match p.enriched_at() {
        None => true,
        Some(at) => (Utc::now() - at).num_days() >= ENRICHMENT_TTL_DAYS,
    }
}

#[cfg(test)]
#[path = "tests/get_credits.rs"]
mod tests;
