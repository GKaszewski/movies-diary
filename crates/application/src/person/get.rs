use crate::context::AppContext;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Person, PersonId},
};

const ENRICHMENT_TTL_DAYS: i64 = 90;

pub async fn execute(ctx: &AppContext, id: PersonId) -> Result<Option<Person>, DomainError> {
    let person = ctx.repos.person_query.get_by_id(&id).await?;
    if let Some(ref p) = person
        && should_enrich(p)
    {
        let _ = ctx
            .services
            .event_publisher
            .publish(&DomainEvent::PersonEnrichmentRequested {
                person_id: id,
                external_person_id: p.external_id().value().to_string(),
            })
            .await;
    }
    Ok(person)
}

fn should_enrich(p: &Person) -> bool {
    match p.enriched_at() {
        None => true,
        Some(at) => (Utc::now() - at).num_days() >= ENRICHMENT_TTL_DAYS,
    }
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
