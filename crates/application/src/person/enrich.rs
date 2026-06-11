use crate::context::AppContext;
use domain::{
    errors::DomainError,
    models::{PersonEnrichmentData, PersonId},
};

pub async fn execute(
    ctx: &AppContext,
    person_id: PersonId,
    data: PersonEnrichmentData,
) -> Result<(), DomainError> {
    ctx.repos.person_command.update_enrichment(&person_id, &data).await
}
