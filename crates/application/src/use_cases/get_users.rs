use domain::{errors::DomainError, models::UserSummary};
use crate::{context::AppContext, queries::GetUsersQuery};

pub async fn execute(
    ctx: &AppContext,
    _query: GetUsersQuery,
) -> Result<Vec<UserSummary>, DomainError> {
    ctx.user_repository.list_with_stats().await
}
