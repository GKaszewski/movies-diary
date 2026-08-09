use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;
use domain::value_objects::WrapUpId;

use crate::wrapup::deps::GetWrapUpDeps;

pub async fn execute(
    deps: &GetWrapUpDeps,
    id: WrapUpId,
) -> Result<Option<WrapUpRecord>, DomainError> {
    deps.wrapup_repo.get_by_id(&id).await
}

#[cfg(test)]
#[path = "tests/get_wrapup.rs"]
mod tests;
