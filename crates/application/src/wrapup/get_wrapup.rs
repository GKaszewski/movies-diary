use std::sync::Arc;

use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;
use domain::ports::WrapUpRepository;
use domain::value_objects::WrapUpId;

pub async fn execute(
    wrapup_repo: Arc<dyn WrapUpRepository>,
    id: WrapUpId,
) -> Result<Option<WrapUpRecord>, DomainError> {
    wrapup_repo.get_by_id(&id).await
}

#[cfg(test)]
#[path = "tests/get_wrapup.rs"]
mod tests;
