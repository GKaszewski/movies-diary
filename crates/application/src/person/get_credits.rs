use crate::context::AppContext;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Person, PersonCredits, PersonId},
};

const ENRICHMENT_TTL_DAYS: i64 = 90;

pub async fn execute(ctx: &AppContext, id: PersonId) -> Result<PersonCredits, DomainError> {
    let credits = ctx.repos.person_query.get_credits(&id).await?;
    if should_enrich(&credits.person) {
        let _ = ctx.services.event_publisher.publish(
            &DomainEvent::PersonEnrichmentRequested {
                person_id: id,
                external_person_id: credits.person.external_id().value().to_string(),
            },
        ).await;
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
