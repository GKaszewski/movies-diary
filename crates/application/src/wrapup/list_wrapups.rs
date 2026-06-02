use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::wrapup::WrapUpRecord;

use crate::context::AppContext;

pub struct ListWrapUpsQuery {
    pub user_id: Option<Uuid>,
}

pub async fn execute(
    ctx: &AppContext,
    query: ListWrapUpsQuery,
) -> Result<Vec<WrapUpRecord>, DomainError> {
    match query.user_id {
        Some(uid) => ctx.repos.wrapup_repo.list_for_user(uid).await,
        None => ctx.repos.wrapup_repo.list_global().await,
    }
}
