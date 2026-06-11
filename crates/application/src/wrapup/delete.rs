use std::sync::Arc;

use domain::errors::DomainError;
use domain::ports::WrapUpRepository;
use domain::value_objects::WrapUpId;

pub async fn execute(
    wrapup_repo: Arc<dyn WrapUpRepository>,
    id: WrapUpId,
) -> Result<(), DomainError> {
    wrapup_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;

    wrapup_repo.delete(&id).await
}

#[cfg(test)]
#[path = "tests/delete.rs"]
mod tests;
