use std::sync::Arc;

use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;
use domain::ports::WrapUpRepository;

pub struct ListWrapUpsQuery {
    pub user_id: Option<Uuid>,
}

pub async fn execute(
    wrapup_repo: Arc<dyn WrapUpRepository>,
    query: ListWrapUpsQuery,
) -> Result<Vec<WrapUpRecord>, DomainError> {
    match query.user_id {
        Some(uid) => wrapup_repo.list_for_user(uid).await,
        None => wrapup_repo.list_global().await,
    }
}

#[cfg(test)]
#[path = "tests/list_wrapups.rs"]
mod tests;
