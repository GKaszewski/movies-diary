use crate::context::AppContext;
use domain::{
    errors::DomainError,
    models::{Person, PersonId},
};

pub async fn execute(ctx: &AppContext, id: PersonId) -> Result<Option<Person>, DomainError> {
    ctx.person_query.get_by_id(&id).await
}
