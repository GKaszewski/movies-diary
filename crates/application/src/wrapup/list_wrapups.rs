use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;

use crate::wrapup::deps::ListWrapUpsDeps;

pub struct ListWrapUpsQuery {
    pub user_id: Option<Uuid>,
}

pub async fn execute(
    deps: &ListWrapUpsDeps,
    query: ListWrapUpsQuery,
) -> Result<Vec<WrapUpRecord>, DomainError> {
    match query.user_id {
        Some(uid) => deps.wrapup_repo.list_for_user(uid).await,
        None => deps.wrapup_repo.list_global().await,
    }
}

#[cfg(test)]
#[path = "tests/list_wrapups.rs"]
mod tests;
