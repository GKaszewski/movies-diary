use crate::context::AppContext;
use chrono::Utc;
use domain::{
    errors::DomainError,
    models::PersonId,
};

const STALENESS_DAYS: i64 = 90;

pub async fn execute(ctx: &AppContext, person_id: PersonId, external_id: &str) -> Result<(), DomainError> {
    if let Some(person) = ctx.repos.person_query.get_by_id(&person_id).await?
        && let Some(at) = person.enriched_at()
        && (Utc::now() - at).num_days() < STALENESS_DAYS
    {
        tracing::debug!(person_id = %person_id.value(), "person enrichment still fresh");
        return Ok(());
    }

    let client = ctx.services.person_enrichment.as_ref().ok_or_else(|| {
        DomainError::InfrastructureError("person enrichment client not configured".into())
    })?;

    let data = match client.fetch_details(external_id).await {
        Ok(d) => d,
        Err(DomainError::NotFound(msg)) => {
            tracing::warn!("TMDb person lookup found nothing: {msg}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    ctx.repos.person_command.update_enrichment(&person_id, &data).await?;
    tracing::info!(person_id = %person_id.value(), "person enriched");
    Ok(())
}
