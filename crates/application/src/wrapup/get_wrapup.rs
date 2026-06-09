use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;
use domain::value_objects::WrapUpId;

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, id: WrapUpId) -> Result<Option<WrapUpRecord>, DomainError> {
    ctx.repos.wrapup_repo.get_by_id(&id).await
}

#[cfg(test)]
#[path = "tests/get_wrapup.rs"]
mod tests;
