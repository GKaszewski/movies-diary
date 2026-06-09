use domain::errors::DomainError;
use domain::value_objects::WrapUpId;

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, id: WrapUpId) -> Result<(), DomainError> {
    ctx.repos
        .wrapup_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;

    ctx.repos.wrapup_repo.delete(&id).await
}

#[cfg(test)]
#[path = "tests/delete.rs"]
mod tests;
