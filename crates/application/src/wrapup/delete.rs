use domain::errors::DomainError;
use domain::value_objects::WrapUpId;

use crate::wrapup::deps::DeleteWrapUpDeps;

pub async fn execute(deps: &DeleteWrapUpDeps, id: WrapUpId) -> Result<(), DomainError> {
    deps.wrapup_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;

    deps.wrapup_repo.delete(&id).await
}

#[cfg(test)]
#[path = "tests/delete.rs"]
mod tests;
