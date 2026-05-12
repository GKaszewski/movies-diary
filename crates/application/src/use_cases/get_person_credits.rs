use domain::{errors::DomainError, models::{PersonCredits, PersonId}};
use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, id: PersonId) -> Result<PersonCredits, DomainError> {
    ctx.person_query.get_credits(&id).await
}
