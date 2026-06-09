use crate::context::AppContext;
use domain::{
    errors::DomainError,
    models::{PersonCredits, PersonId},
};

pub async fn execute(ctx: &AppContext, id: PersonId) -> Result<PersonCredits, DomainError> {
    ctx.repos.person_query.get_credits(&id).await
}

#[cfg(test)]
#[path = "tests/get_credits.rs"]
mod tests;
